use super::*;
use hbb_common::{
    message_proto::{EncodedVideoFrame, EncodedVideoFrames},
    tokio, Stream,
};

fn video_message() -> Message {
    let mut frame = VideoFrame::new();
    frame.set_h264s(EncodedVideoFrames {
        frames: vec![
            EncodedVideoFrame {
                data: vec![0; 4096].into(),
                ..Default::default()
            };
            2
        ],
        ..Default::default()
    });
    let mut msg = Message::new();
    msg.set_video_frame(frame);
    msg
}

#[tokio::test]
async fn blocked_stream_is_observed_before_send_returns_and_monitor_stops() {
    let (tx, mut observations) = tokio::sync::mpsc::unbounded_channel();
    let monitor = Monitor::start(
        "connection id=1652".into(),
        Duration::from_millis(10),
        move |_, snapshot, _| {
            let _ = tx.send(snapshot);
        },
    );
    let shared = Arc::downgrade(monitor.shared.as_ref().unwrap());
    monitor.arm_timer("probe", Instant::now());
    let (sender, receiver) = tokio::io::duplex(64);
    let mut stream = Stream::Tcp(hbb_common::tcp::FramedStream::from(
        sender,
        "127.0.0.1:1".parse().unwrap(),
    ));
    let msg = video_message();
    let queued = Instant::now() - Duration::from_millis(200);
    let mut span = monitor.message("send", &msg, Some(queued));
    let mut send = Box::pin(stream.send(&msg));
    let snapshot = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            tokio::select! {
                biased;
                result = &mut send => panic!("send should block on the full buffer: {result:?}"),
                snapshot = observations.recv() => {
                    let snapshot = snapshot.unwrap();
                    if snapshot.active.is_some() {
                        break snapshot;
                    }
                },
            }
        }
    })
    .await
    .unwrap();
    let active = snapshot.active.unwrap();
    assert_eq!(active.stage, "send");
    assert_eq!(active.kind, "video");
    assert!(active.queue_age(Instant::now()) >= Duration::from_millis(200));
    assert!(snapshot.timer_overdue[0].1 > Duration::ZERO);
    assert!(
        snapshot.stats.is_empty(),
        "the pending send has no completion yet"
    );

    let drain = tokio::spawn(async move {
        let mut peer = Stream::Tcp(hbb_common::tcp::FramedStream::from(
            receiver,
            "127.0.0.1:2".parse().unwrap(),
        ));
        peer.next().await.unwrap().unwrap()
    });
    tokio::time::timeout(Duration::from_secs(3), &mut send)
        .await
        .unwrap()
        .unwrap();
    span.finish("ok");
    drop(span);
    assert!(!drain.await.unwrap().is_empty());
    let completed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot = observations.recv().await.unwrap();
            if snapshot.stats.contains_key(&("send", "video")) {
                break snapshot;
            }
        }
    })
    .await
    .unwrap();
    assert!(completed.active.is_none());
    assert_eq!(completed.stats[&("send", "video")].ok, 1);
    assert!(completed.stats[&("send", "video")].queue_max >= Duration::from_millis(200));
    drop(monitor);
    tokio::time::timeout(Duration::from_secs(3), async {
        while observations.recv().await.is_some() {}
        while shared.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        shared.upgrade().is_none(),
        "observer must release connection state"
    );
}

#[test]
fn nested_send_restores_receive_stage_and_counts_encoded_frames_not_messages() {
    // No observer is needed for deterministic interval accounting.
    let monitor = Monitor {
        shared: Some(Arc::new(Shared {
            scope: "test".into(),
            state: Mutex::new(State::default()),
        })),
        _stop: None,
    };
    let outer = monitor.enter("receive_handler");
    let msg = video_message();
    let mut send = monitor.message("send", &msg, None);
    send.finish("error");
    drop(send);
    let Some(message::Union::VideoFrame(vf)) = &msg.union else {
        panic!()
    };
    monitor.encoded(vf);
    monitor.waiting(&HashSet::from([70, 1652]), &HashSet::from([70]));
    let first = monitor.shared.as_ref().unwrap().snapshot(Instant::now());
    assert_eq!(first.active.unwrap().stage, "receive_handler");
    assert_eq!(first.stats[&("send", "video")].errors, 1);
    assert_eq!(first.stats[&("send", "video")].count, 1);
    assert_eq!(first.encoded_frames, 2);
    assert_eq!(first.encoded_bytes, 8192);
    assert_eq!(first.waiting_for, vec![1652]);
    drop(outer);
    let second = monitor.shared.as_ref().unwrap().snapshot(Instant::now());
    assert!(second.active.is_none());
    assert_eq!(second.encoded_frames, 0);
    assert!(!second.stats.contains_key(&("send", "video")));
}

#[test]
fn disabled_monitor_does_not_allocate_state() {
    let monitor = Monitor::default();
    let mut span = monitor.message("send", &video_message(), Some(Instant::now()));
    assert!(span.shared.is_none());
    assert!(span.active.is_none());
    span.finish("ok");
}
