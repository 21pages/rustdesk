use super::{qos_diagnostics, video_service, Connection, Message};
use hbb_common::{
    log,
    message_proto::message,
    tcp::FramedStream,
    tokio::{
        self,
        io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf},
        sync::Notify,
        task::JoinHandle,
        time::{timeout, timeout_at, Duration, Instant},
    },
    ResultType, Stream,
};
use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

#[cfg(test)]
mod tests;

const MAX_PACKETS: usize = 128;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const AUDIO_MAX_AGE: Duration = Duration::from_millis(150);

pub(super) struct RemoteWriter {
    queue: Arc<Queue>,
    task: Option<JoinHandle<io::Result<()>>>,
    timeout: Duration,
}

#[derive(Default)]
struct State {
    packets: VecDeque<Packet>,
    bytes: usize,
    in_flight: Option<Instant>,
    closed: bool,
    error: Option<String>,
    next_video: Option<Video>,
    audio_dropped: usize,
}

#[derive(Default)]
struct Queue {
    state: Mutex<State>,
    ready: Notify,
}

struct Packet {
    bytes: Vec<u8>,
    queued: Instant,
    video: Option<Video>,
}

struct Video {
    produced: Instant,
    display: Option<usize>,
    ack_required: bool,
}

impl Queue {
    fn check(state: &State) -> io::Result<()> {
        if state.closed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                state.error.as_deref().unwrap_or("remote writer closed"),
            ));
        }
        Ok(())
    }

    fn push(&self, bytes: &[u8]) -> io::Result<usize> {
        let mut state = self.state.lock().unwrap();
        Self::check(&state)?;
        // A single large frame is allowed when empty. Count the active write too,
        // so taking a packet out of the queue cannot hide socket backpressure.
        if state.packets.len() + usize::from(state.in_flight.is_some()) >= MAX_PACKETS
            || (state.bytes > 0 && bytes.len() > MAX_BYTES.saturating_sub(state.bytes))
        {
            state.closed = true;
            state.error = Some("remote writer queue limit exceeded".to_owned());
            self.ready.notify_one();
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "remote writer queue limit exceeded",
            ));
        }
        let video = state.next_video.take();
        state.packets.push_back(Packet {
            bytes: bytes.to_vec(),
            queued: Instant::now(),
            video,
        });
        state.bytes += bytes.len();
        self.ready.notify_one();
        Ok(bytes.len())
    }

    fn drop_audio(&self, produced: Instant) -> bool {
        let mut state = self.state.lock().unwrap();
        let oldest = state
            .in_flight
            .or_else(|| state.packets.front().map(|packet| packet.queued));
        let drop = produced.elapsed() >= AUDIO_MAX_AGE
            || oldest.map_or(false, |queued| queued.elapsed() >= AUDIO_MAX_AGE)
            || state.packets.len() >= MAX_PACKETS / 2
            || state.bytes >= MAX_BYTES / 2;
        if drop {
            state.audio_dropped += 1;
        }
        drop
    }
}

// Keep Framed's codec, buffered reads, and both encryption counters in place.
// Only socket writes move to the worker; flush here means accepted by its queue.
struct QueuedIo<R> {
    reader: R,
    queue: Arc<Queue>,
}

impl<R: AsyncRead + Unpin> AsyncRead for QueuedIo<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl<R: Unpin> AsyncWrite for QueuedIo<R> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.queue.push(bytes))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Queue::check(&self.queue.state.lock().unwrap()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.queue.state.lock().unwrap().closed = true;
        self.queue.ready.notify_one();
        Poll::Ready(Ok(()))
    }
}

impl RemoteWriter {
    fn start(stream: &mut FramedStream, id: i32) -> Self {
        let queue = Arc::new(Queue::default());
        let socket = std::mem::replace(
            &mut stream.0.get_mut().0,
            Box::new(QueuedIo {
                reader: tokio::io::empty(),
                queue: queue.clone(),
            }),
        );
        let (reader, writer) = tokio::io::split(socket);
        stream.0.get_mut().0 = Box::new(QueuedIo {
            reader,
            queue: queue.clone(),
        });
        let send_timeout = Duration::from_millis(super::SEND_TIMEOUT_VIDEO);
        let worker_queue = queue.clone();
        let task = tokio::spawn(async move {
            let result = write_loop(writer, &worker_queue, id, send_timeout).await;
            let mut state = worker_queue.state.lock().unwrap();
            state.closed = true;
            if let Err(err) = &result {
                state.error = Some(err.to_string());
            }
            result
        });
        log::info!(target: "rustdesk::video_qos",
            "qos_writer_mode conn={} enabled=true transport=tcp max_packets={} max_bytes={} audio_max_age_ms={} complete=true",
            id, MAX_PACKETS, MAX_BYTES, AUDIO_MAX_AGE.as_millis());
        Self {
            queue,
            task: Some(task),
            timeout: send_timeout,
        }
    }

    pub(super) fn file_ready(&self) -> bool {
        let state = self.queue.state.lock().unwrap();
        state.packets.len() < 16 && state.bytes < 256 * 1024
    }

    async fn send_video(
        &self,
        stream: &mut FramedStream,
        message: &Message,
        produced: Instant,
        ack_required: bool,
    ) -> ResultType<()> {
        use hbb_common::futures::SinkExt;

        // Buffered bytes from before activation must not consume this frame's notification.
        stream.0.flush().await?;
        self.queue.state.lock().unwrap().next_video = Some(Video {
            produced,
            display: match &message.union {
                Some(message::Union::VideoFrame(frame)) => Some(frame.display as usize),
                _ => None,
            },
            ack_required,
        });
        stream.send(message).await
    }

    async fn finish(&mut self) -> io::Result<()> {
        self.queue.state.lock().unwrap().closed = true;
        self.queue.ready.notify_one();
        if let Some(task) = self.task.as_mut() {
            let result = timeout(self.timeout, task).await.map_err(|_| {
                io::Error::new(io::ErrorKind::TimedOut, "remote writer drain timed out")
            })?;
            self.task.take();
            result.map_err(io::Error::other)??;
        }
        Ok(())
    }
}

impl Drop for RemoteWriter {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

pub(super) async fn stopped(writer: &mut Option<RemoteWriter>) -> io::Error {
    let Some(writer) = writer else {
        return std::future::pending().await;
    };
    let Some(task) = writer.task.as_mut() else {
        return std::future::pending().await;
    };
    let result = task.await;
    writer.task.take();
    match result {
        Ok(Err(err)) => err,
        Err(err) => io::Error::other(err),
        Ok(Ok(())) => io::Error::new(io::ErrorKind::BrokenPipe, "remote writer stopped"),
    }
}

async fn write_loop<W: AsyncWrite + Unpin>(
    mut writer: W,
    queue: &Queue,
    id: i32,
    send_timeout: Duration,
) -> io::Result<()> {
    let diagnostics = qos_diagnostics::Monitor::new(|| format!("writer id={id}"));
    let mut window = Instant::now();
    loop {
        let ready = queue.ready.notified();
        let packet = {
            let mut state = queue.state.lock().unwrap();
            if let Some(err) = &state.error {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, err.clone()));
            }
            let packet = state.packets.pop_front();
            if let Some(packet) = &packet {
                state.in_flight = Some(packet.queued);
            } else if state.closed {
                break;
            }
            packet
        };
        let Some(packet) = packet else {
            ready.await;
            continue;
        };
        if packet.queued.elapsed() >= send_timeout {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "remote writer packet expired in queue",
            ));
        }
        if let Some(video) = &packet.video {
            if !video.ack_required {
                if let Some(display) = video.display {
                    video_service::notify_video_frame_fetched(
                        display,
                        id,
                        Some(video.produced.into()),
                    );
                }
            }
        }
        let mut diag_send = diagnostics.socket_write(
            packet.video.as_ref().and_then(|video| video.display),
            packet
                .video
                .as_ref()
                .map_or(packet.queued, |video| video.produced)
                .into_std(),
        );
        // Include queue residence in the deadline; otherwise a bounded queue could
        // still retain many sequential send-timeout intervals of old traffic.
        let result = timeout_at(packet.queued + send_timeout, async {
            writer.write_all(&packet.bytes).await?;
            writer.flush().await
        })
        .await
        .unwrap_or_else(|_| {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "remote writer send timed out",
            ))
        });
        diag_send.finish(if result.is_ok() { "ok" } else { "error" });
        let (pending, pending_bytes, audio_dropped) = {
            let mut state = queue.state.lock().unwrap();
            state.bytes -= packet.bytes.len();
            state.in_flight = None;
            (state.packets.len(), state.bytes, state.audio_dropped)
        };
        if diagnostics.is_enabled()
            && (window.elapsed() >= Duration::from_secs(1) || result.is_err())
        {
            log::info!(
                "qos_writer conn={} pending={} pending_bytes={} audio_dropped_total={} failed={}",
                id,
                pending,
                pending_bytes,
                audio_dropped,
                result.is_err()
            );
            window = Instant::now();
        }
        result?;
    }
    timeout(send_timeout, writer.shutdown())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "remote writer shutdown timed out"))?
}

impl Connection {
    pub(super) fn start_remote_writer(&mut self) {
        if self.remote_writer.is_none() && self.is_authed_remote_conn() {
            if let Stream::Tcp(stream) = &mut self.stream {
                self.remote_writer = Some(RemoteWriter::start(stream, self.inner.id));
            }
        }
    }

    pub(super) fn drop_stale_audio(&self, message: &Message, produced: Instant) -> bool {
        matches!(message.union, Some(message::Union::AudioFrame(_)))
            && self
                .remote_writer
                .as_ref()
                .map_or(false, |writer| writer.queue.drop_audio(produced))
    }

    pub(super) async fn send_remote_video(
        &mut self,
        message: &Message,
        produced: Instant,
    ) -> ResultType<()> {
        let mut diag_send = self
            .diagnostics
            .message("enqueue", message, Some(produced.into_std()));
        let result = match (&self.remote_writer, &mut self.stream) {
            (Some(writer), Stream::Tcp(stream)) => {
                writer
                    .send_video(stream, message, produced, self.video_ack_required)
                    .await
            }
            _ => self.stream.send(message).await,
        };
        diag_send.finish(if result.is_ok() { "ok" } else { "error" });
        result
    }

    pub(super) async fn finish_remote_writer(&mut self) {
        if let Some(mut writer) = self.remote_writer.take() {
            if let Err(err) = writer.finish().await {
                log::debug!("#{} remote writer stopped: {}", self.inner.id, err);
            }
        }
    }
}
