use super::*;
use hbb_common::{
    bytes_codec::BytesCodec,
    futures::{self, SinkExt},
    sodiumoxide::crypto::secretbox::Key,
    tokio::io::{duplex, AsyncReadExt},
    tokio_util::codec::Encoder,
};

fn framed(socket: tokio::io::DuplexStream) -> FramedStream {
    FramedStream::from(socket, "127.0.0.1:1".parse().unwrap())
}

async fn wait_for(mut ready: impl FnMut() -> bool) {
    timeout(Duration::from_secs(2), async {
        while !ready() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn blocked_write_keeps_reads_and_timers_running() {
    let (socket, peer) = duplex(64);
    let (mut stream, mut peer) = (framed(socket), framed(peer));
    let writer = RemoteWriter::start(&mut stream, -1);
    timeout(Duration::from_millis(100), stream.send_raw(vec![7; 4096]))
        .await
        .unwrap()
        .unwrap();
    wait_for(|| writer.queue.state.lock().unwrap().in_flight.is_some()).await;

    peer.send_raw(b"input".to_vec()).await.unwrap();
    let input = timeout(Duration::from_millis(100), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(&input[..], b"input");
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_millis(10)) => {},
        result = stream.next() => panic!("unexpected input: {result:?}"),
    }
    assert!(writer.queue.state.lock().unwrap().in_flight.is_some());
    assert!(!writer.task.as_ref().unwrap().is_finished());
}

#[tokio::test]
async fn activation_preserves_partial_decoder_and_buffered_payload() {
    let (socket, mut peer) = duplex(1024);
    let mut stream = framed(socket);
    let payload = vec![9; 512];
    let mut wire = bytes::BytesMut::new();
    BytesCodec::new()
        .encode(bytes::Bytes::from(payload.clone()), &mut wire)
        .unwrap();
    peer.write_all(&wire[..10]).await.unwrap();
    // Reading the header advances the codec even though the frame is incomplete.
    assert!(futures::poll!(Box::pin(stream.next())).is_pending());
    assert!(!stream.0.read_buffer().is_empty());
    let _writer = RemoteWriter::start(&mut stream, -2);
    peer.write_all(&wire[10..]).await.unwrap();
    assert_eq!(&stream.next().await.unwrap().unwrap()[..], payload);
}

#[tokio::test]
async fn activation_preserves_encryption_order_and_pending_writes() {
    let (socket, peer) = duplex(1024);
    let (mut stream, mut peer) = (framed(socket), framed(peer));
    stream.set_key(Key([23; 32]));
    peer.set_key(Key([23; 32]));
    stream.send_raw(b"login".to_vec()).await.unwrap();
    assert_eq!(&peer.next().await.unwrap().unwrap()[..], b"login");
    peer.send_raw(b"before split".to_vec()).await.unwrap();
    peer.send_raw(b"buffered".to_vec()).await.unwrap();
    assert_eq!(&stream.next().await.unwrap().unwrap()[..], b"before split");
    assert!(!stream.0.read_buffer().is_empty());

    // A buffered ciphertext must precede all new ciphertext after activation.
    let pending = stream.2.as_mut().unwrap().enc(b"pending");
    stream.0.feed(bytes::Bytes::from(pending)).await.unwrap();
    let mut writer = RemoteWriter::start(&mut stream, -3);
    stream.0.flush().await.unwrap();
    assert!(writer.queue.drop_audio(Instant::now() - AUDIO_MAX_AGE));
    stream.send_raw(b"video".to_vec()).await.unwrap();
    stream.send_raw(b"control".to_vec()).await.unwrap();
    assert_eq!(&stream.next().await.unwrap().unwrap()[..], b"buffered");
    peer.send_raw(b"after split".to_vec()).await.unwrap();
    assert_eq!(&stream.next().await.unwrap().unwrap()[..], b"after split");
    let (result, ()) = tokio::join!(writer.finish(), async {
        for expected in [b"pending".as_slice(), b"video", b"control"] {
            assert_eq!(&peer.next().await.unwrap().unwrap()[..], expected);
        }
        assert!(peer.next().await.is_none());
    });
    result.unwrap();
}

#[tokio::test]
async fn video_waits_behind_active_write_and_drain_preserves_close_message() {
    let (socket, peer) = duplex(64);
    let (mut stream, mut peer) = (framed(socket), framed(peer));
    let mut writer = RemoteWriter::start(&mut stream, -4);
    stream.send_raw(vec![1; 4096]).await.unwrap();
    wait_for(|| writer.queue.state.lock().unwrap().in_flight.is_some()).await;
    writer.queue.state.lock().unwrap().next_video = Some(Video {
        produced: Instant::now(),
        display: None,
        ack_required: false,
    });
    stream.send_raw(b"video".to_vec()).await.unwrap();
    tokio::task::yield_now().await;
    {
        let state = writer.queue.state.lock().unwrap();
        assert_eq!(state.packets.len(), 1);
        assert!(state.packets.front().unwrap().video.is_some());
        assert!(state.bytes >= 4096);
    }
    stream.send_raw(b"close reason".to_vec()).await.unwrap();
    let (result, ()) = tokio::join!(writer.finish(), async {
        assert_eq!(&peer.next().await.unwrap().unwrap()[..], vec![1; 4096]);
        assert_eq!(&peer.next().await.unwrap().unwrap()[..], b"video");
        assert_eq!(&peer.next().await.unwrap().unwrap()[..], b"close reason");
        assert!(peer.next().await.is_none());
    });
    result.unwrap();
}

#[tokio::test]
async fn video_notification_tracks_writer_dequeue_and_respects_viewer_ack() {
    use hbb_common::message_proto::{EncodedVideoFrame, EncodedVideoFrames, VideoFrame};

    for ack_required in [false, true] {
        let display = i32::MAX - i32::from(ack_required);
        let (_cleanup, fetched) = video_service::test_frame_notifications(display as usize);
        let mut fetched = fetched.lock().await;
        let (socket, peer) = duplex(64);
        let (mut stream, mut peer) = (framed(socket), framed(peer));
        // Leave a complete framed message buffered before activating the worker.
        stream
            .0
            .feed(bytes::Bytes::from(vec![1; 4096]))
            .await
            .unwrap();
        let mut writer = RemoteWriter::start(&mut stream, -40);
        let produced = Instant::now();
        let mut frame = VideoFrame::new();
        frame.display = display;
        frame.set_h264s(EncodedVideoFrames {
            frames: vec![EncodedVideoFrame {
                data: vec![2; 4096].into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        let mut msg = Message::new();
        msg.set_video_frame(frame);
        writer
            .send_video(&mut stream, &msg, produced, ack_required)
            .await
            .unwrap();
        wait_for(|| writer.queue.state.lock().unwrap().in_flight.is_some()).await;
        assert!(
            fetched.try_recv().is_err(),
            "enqueue must not release capture"
        );
        assert!(writer
            .queue
            .state
            .lock()
            .unwrap()
            .packets
            .front()
            .unwrap()
            .video
            .is_some());

        stream.send_raw(b"close reason".to_vec()).await.unwrap();
        assert_eq!(&peer.next().await.unwrap().unwrap()[..], vec![1; 4096]);
        wait_for(|| writer.queue.state.lock().unwrap().packets.len() == 1).await;
        if ack_required {
            assert!(fetched.try_recv().is_err());
        } else {
            assert_eq!(
                timeout(Duration::from_secs(1), fetched.recv())
                    .await
                    .unwrap(),
                Some((-40, Some(produced.into_std())))
            );
        }
        assert!(writer.queue.state.lock().unwrap().in_flight.is_some());
        let (result, ()) = tokio::join!(writer.finish(), async {
            use hbb_common::protobuf::Message as _;
            assert_eq!(
                &peer.next().await.unwrap().unwrap()[..],
                msg.write_to_bytes().unwrap()
            );
            assert_eq!(&peer.next().await.unwrap().unwrap()[..], b"close reason");
            assert!(peer.next().await.is_none());
        });
        result.unwrap();
        assert!(
            fetched.try_recv().is_err(),
            "no extra local acknowledgement"
        );
    }
}

#[test]
fn queue_limits_include_bytes_and_active_packet() {
    let queue = Queue::default();
    for _ in 0..MAX_PACKETS {
        queue.push(&[1]).unwrap();
    }
    assert!(queue.push(&[2]).is_err());
    assert_eq!(queue.state.lock().unwrap().packets.len(), MAX_PACKETS);

    let queue = Queue::default();
    queue.push(&vec![1; MAX_BYTES]).unwrap();
    {
        let mut state = queue.state.lock().unwrap();
        state.packets.pop_front();
        state.in_flight = Some(Instant::now());
    }
    assert!(queue.push(&[2]).is_err());
    assert_eq!(queue.state.lock().unwrap().bytes, MAX_BYTES);

    let queue = Queue::default();
    queue.push(&vec![1; MAX_BYTES + 1]).unwrap();
    assert!(queue.push(&[2]).is_err());
    assert_eq!(queue.state.lock().unwrap().packets.len(), 1);
}

#[test]
fn stale_audio_is_rejected_before_encryption_and_recovers_after_backlog() {
    let queue = Queue::default();
    assert!(!queue.drop_audio(Instant::now()));
    assert!(queue.drop_audio(Instant::now() - AUDIO_MAX_AGE));
    queue.state.lock().unwrap().in_flight = Some(Instant::now() - AUDIO_MAX_AGE);
    assert!(queue.drop_audio(Instant::now()));
    queue.state.lock().unwrap().in_flight = None;
    assert!(!queue.drop_audio(Instant::now()));
    assert_eq!(queue.state.lock().unwrap().audio_dropped, 2);
}

#[tokio::test]
async fn stalled_socket_times_out_including_queue_residence() {
    let (writer, _peer) = duplex(1);
    let queue = Queue::default();
    queue.push(&[1, 2, 3]).unwrap();
    // Already spent the send budget waiting behind another packet.
    queue
        .state
        .lock()
        .unwrap()
        .packets
        .front_mut()
        .unwrap()
        .queued = Instant::now() - Duration::from_secs(1);
    let err = timeout(
        Duration::from_millis(100),
        write_loop(writer, &queue, -5, Duration::from_millis(20)),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}

#[tokio::test]
async fn expired_packet_is_not_written_even_when_socket_is_writable() {
    let (writer, mut peer) = duplex(64);
    let queue = Queue::default();
    queue.push(&[1, 2, 3]).unwrap();
    queue
        .state
        .lock()
        .unwrap()
        .packets
        .front_mut()
        .unwrap()
        .queued = Instant::now() - Duration::from_secs(1);
    let err = write_loop(writer, &queue, -11, Duration::from_millis(20))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    let mut received = Vec::new();
    peer.read_to_end(&mut received).await.unwrap();
    assert!(received.is_empty());
}

#[tokio::test]
async fn partial_write_timeout_stops_before_the_next_packet() {
    let (writer, mut peer) = duplex(1);
    let queue = Queue::default();
    queue.push(&[1, 2, 3]).unwrap();
    queue.push(&[4, 5, 6]).unwrap();
    let err = timeout(
        Duration::from_secs(1),
        write_loop(writer, &queue, -12, Duration::from_millis(20)),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    let mut received = Vec::new();
    peer.read_to_end(&mut received).await.unwrap();
    assert_eq!(received, vec![1]);
    assert_eq!(queue.state.lock().unwrap().packets.len(), 1);
}

#[tokio::test]
async fn socket_failure_wakes_connection_and_drop_cancels_worker() {
    let (socket, peer) = duplex(16);
    let mut stream = framed(socket);
    let mut writer = Some(RemoteWriter::start(&mut stream, -6));
    drop(peer);
    stream.send_raw(vec![1; 64]).await.unwrap();
    let err = timeout(Duration::from_secs(1), stopped(&mut writer))
        .await
        .unwrap();
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    assert!(writer.as_ref().unwrap().task.is_none());
    assert!(stream.send_raw(vec![2; 64]).await.is_err());

    let (socket, _peer) = duplex(16);
    let mut stream = framed(socket);
    let writer = RemoteWriter::start(&mut stream, -7);
    let abort = writer.task.as_ref().unwrap().abort_handle();
    stream.send_raw(vec![1; 64]).await.unwrap();
    wait_for(|| writer.queue.state.lock().unwrap().in_flight.is_some()).await;
    drop(writer);
    wait_for(|| abort.is_finished()).await;
}

#[tokio::test]
async fn drain_timeout_does_not_leave_a_worker_running() {
    let (socket, _peer) = duplex(1);
    let mut stream = framed(socket);
    let mut writer = RemoteWriter::start(&mut stream, -10);
    writer.timeout = Duration::from_millis(20);
    let abort = writer.task.as_ref().unwrap().abort_handle();
    stream.send_raw(vec![1; 64]).await.unwrap();
    let err = timeout(Duration::from_millis(200), writer.finish())
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    drop(writer);
    wait_for(|| abort.is_finished()).await;
}

#[tokio::test]
async fn slow_writer_smoke() {
    for stall_ms in [0, 20, 150, 800, 1500, 3000] {
        let (socket, peer) = duplex(64);
        let (mut stream, mut peer) = (framed(socket), framed(peer));
        let mut writer = RemoteWriter::start(&mut stream, -8);
        stream.send_raw(vec![8; 4096]).await.unwrap();
        wait_for(|| writer.queue.state.lock().unwrap().in_flight.is_some()).await;
        let start = Instant::now();
        let mut ticks = 0;
        let mut admitted = 0;
        let mut interval = tokio::time::interval(Duration::from_millis(10));
        while start.elapsed() < Duration::from_millis(stall_ms) {
            interval.tick().await;
            peer.send_raw(b"input".to_vec()).await.unwrap();
            let reply = timeout(Duration::from_millis(100), stream.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(&reply[..], b"input");
            ticks += 1;
            if !writer.queue.drop_audio(Instant::now()) {
                stream.send_raw(b"audio".to_vec()).await.unwrap();
                admitted += 1;
            }
        }
        let (pending, dropped) = {
            let state = writer.queue.state.lock().unwrap();
            assert!(state.bytes < MAX_BYTES);
            assert!(state.packets.len() < MAX_PACKETS);
            (state.packets.len(), state.audio_dropped)
        };
        if stall_ms >= 150 {
            assert!(dropped > 0);
        }
        let (result, ()) = tokio::join!(writer.finish(), async {
            assert_eq!(&peer.next().await.unwrap().unwrap()[..], vec![8; 4096]);
            for _ in 0..admitted {
                assert_eq!(&peer.next().await.unwrap().unwrap()[..], b"audio");
            }
            assert!(peer.next().await.is_none());
        });
        result.unwrap();
        println!("slow_writer_smoke stall_ms={stall_ms} input_and_timer_ticks={ticks} queued_audio={pending} dropped_audio={dropped} ordered_drain=pass");
    }
}

#[tokio::test]
async fn printer_job_uses_existing_stream_api_after_activation() {
    use hbb_common::{fs, message_proto::file_response, protobuf::Message as _};
    let (socket, peer) = duplex(64);
    let (stream, mut peer) = (framed(socket), framed(peer));
    let mut stream = Stream::Tcp(stream);
    let Stream::Tcp(tcp) = &mut stream else {
        unreachable!()
    };
    let mut writer = RemoteWriter::start(tcp, -9);
    let mut jobs = vec![fs::TransferJob::new_read(
        123,
        fs::JobType::Printer,
        String::new(),
        fs::DataSource::MemoryCursor(std::io::Cursor::new(b"print data".to_vec())),
        0,
        false,
        false,
        false,
    )
    .unwrap()];
    while !jobs.is_empty() {
        timeout(
            Duration::from_millis(100),
            fs::handle_read_jobs(&mut jobs, &mut stream),
        )
        .await
        .unwrap()
        .unwrap();
    }
    let (result, ()) = tokio::join!(writer.finish(), async {
        let block = peer.next().await.unwrap().unwrap();
        let block = Message::parse_from_bytes(&block).unwrap();
        let Some(message::Union::FileResponse(response)) = block.union else {
            panic!("missing response")
        };
        let Some(file_response::Union::Block(block)) = response.union else {
            panic!("missing block")
        };
        assert_eq!(&block.data[..], b"print data");
        let done = peer.next().await.unwrap().unwrap();
        let done = Message::parse_from_bytes(&done).unwrap();
        let Some(message::Union::FileResponse(response)) = done.union else {
            panic!("missing response")
        };
        assert!(matches!(
            response.union,
            Some(file_response::Union::Done(_))
        ));
    });
    result.unwrap();
}
