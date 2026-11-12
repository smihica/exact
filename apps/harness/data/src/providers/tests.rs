use super::*;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use ureq::unversioned::transport::LazyBuffers;

const WAIT: Duration = Duration::from_secs(10);
const HEADERS: &[u8] =
    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";

// The peer answers each read explicitly. Only the peer advances time;
// wall-clock bounds detect a stuck test, never decide a request's deadline.
struct Peer {
    clock: Clock,
    reads: mpsc::Receiver<NextTimeout>,
    replies: mpsc::Sender<Option<&'static [u8]>>,
}

impl Peer {
    fn advance(&self, by: Duration) {
        *self.clock.time.as_ref().unwrap().lock().unwrap() += by;
    }

    fn read(&self) -> NextTimeout {
        self.reads
            .recv_timeout(WAIT)
            .expect("reader requested input")
    }

    fn send(&self, mut after: Duration, bytes: &'static [u8]) {
        loop {
            let timeout = self.read();
            assert!(!timeout.after.is_not_happening());
            let slice = *timeout.after;
            if after < slice {
                self.advance(after);
                self.replies.send(Some(bytes)).unwrap();
                return;
            }
            self.advance(slice);
            self.replies.send(None).unwrap();
            after -= slice;
        }
    }

    fn closed(&self) {
        assert!(matches!(
            self.reads.recv_timeout(WAIT),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ));
    }
}

#[derive(Debug)]
struct ScriptedTransport {
    buffers: LazyBuffers,
    reads: mpsc::Sender<NextTimeout>,
    replies: Mutex<mpsc::Receiver<Option<&'static [u8]>>>,
}

impl Transport for ScriptedTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(&mut self, _: usize, _: NextTimeout) -> Result<(), ureq::Error> {
        Ok(())
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, ureq::Error> {
        self.reads.send(timeout).unwrap();
        match self.replies.lock().unwrap().recv_timeout(WAIT) {
            Ok(Some(bytes)) => {
                self.buffers.input_append_buf()[..bytes.len()].copy_from_slice(bytes);
                self.buffers.input_appended(bytes.len());
                Ok(!bytes.is_empty())
            }
            Ok(None) => Err(ureq::Error::Timeout(timeout.reason)),
            // Release the reader even when an assertion ends the test early.
            Err(_) => Ok(false),
        }
    }

    fn is_open(&mut self) -> bool {
        true
    }
}

#[derive(Debug)]
struct ScriptedConnector(Mutex<Option<ScriptedTransport>>);

impl Connector for ScriptedConnector {
    type Out = Box<dyn Transport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        _: Option<()>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        // ureq's clock is private. Check its body deadlines here as well
        // as exercising our idle clock: any of these caps a live stream.
        let timeouts = details.config.timeouts();
        assert_eq!(timeouts.global, None);
        assert_eq!(timeouts.per_call, None);
        assert_eq!(timeouts.recv_response, None);
        assert_eq!(timeouts.recv_body, None);
        Ok(Some(Box::new(self.0.lock().unwrap().take().unwrap())))
    }
}

fn transport() -> (Peer, ScriptedTransport) {
    let (reads_tx, reads) = mpsc::channel();
    let (replies, replies_rx) = mpsc::channel();
    (
        Peer {
            clock: Clock {
                time: Some(Arc::new(Mutex::new(Instant::now()))),
            },
            reads,
            replies,
        },
        ScriptedTransport {
            buffers: LazyBuffers::new(4096, 4096),
            reads: reads_tx,
            replies: Mutex::new(replies_rx),
        },
    )
}

fn request(
    mut each: impl FnMut(Sse) -> bool + Send + 'static,
) -> (Peer, Arc<AtomicBool>, JoinHandle<Result<(), String>>) {
    let (peer, transport) = transport();
    let clock = peer.clock.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let worker = thread::spawn(move || {
        post(
            "http://127.0.0.1/",
            &[],
            &serde_json::json!({}),
            &flag,
            ScriptedConnector(Mutex::new(Some(transport))),
            clock,
            &mut each,
        )
    });
    (peer, cancel, worker)
}

#[test]
fn a_stream_can_outlast_the_idle_limit() {
    let (events_tx, events) = mpsc::channel();
    let (peer, _, worker) = request(move |event| {
        events_tx.send(event.data).unwrap();
        true
    });
    let start = peer.clock.now();
    peer.send(Duration::ZERO, HEADERS);
    for _ in 0..40 {
        peer.send(IDLE / 20, b"data: token\n\n");
        assert_eq!(events.recv_timeout(WAIT).unwrap(), "token");
    }
    peer.send(Duration::ZERO, b"");
    assert_eq!(worker.join().unwrap(), Ok(()));
    peer.closed();
    assert_eq!(peer.clock.now().duration_since(start), 2 * IDLE);
}

#[test]
fn silence_times_out_and_closes_the_connection() {
    for headers in [false, true] {
        let (events_tx, events) = mpsc::channel();
        let (peer, _, worker) = request(move |event| {
            events_tx.send(event.data).unwrap();
            true
        });
        if headers {
            peer.send(Duration::ZERO, HEADERS);
            peer.send(Duration::ZERO, b"data: token\n\ndata: unfinished");
            assert_eq!(events.recv_timeout(WAIT).unwrap(), "token");
        }
        let start = peer.clock.now();
        while peer.clock.now().duration_since(start) < IDLE {
            let timeout = peer.read();
            peer.advance(*timeout.after);
            peer.replies.send(None).unwrap();
        }
        assert!(worker
            .join()
            .unwrap()
            .unwrap_err()
            .contains("no data from the server for 120 s"));
        peer.closed();
    }
}

#[test]
fn a_trickling_partial_line_closes_when_post_returns() {
    let (events_tx, events) = mpsc::channel();
    let (peer, cancel, worker) = request(move |_| {
        events_tx.send(()).unwrap();
        true
    });
    peer.send(Duration::ZERO, HEADERS);
    peer.send(Duration::ZERO, b"data: token\n\n");
    events.recv_timeout(WAIT).unwrap();
    // Never finish another line, but keep every read shorter than SLICE.
    for _ in 0..1200 {
        peer.send(Duration::from_millis(100), b"x");
    }
    peer.read();
    peer.advance(Duration::from_millis(100));
    assert!(worker
        .join()
        .unwrap()
        .unwrap_err()
        .contains("no data from the server for 120 s"));
    assert!(!cancel.load(Ordering::SeqCst));
    peer.replies.send(Some(b"x")).unwrap();
    peer.closed();
}

#[test]
fn ending_the_event_callback_stops_a_partial_line_reader() {
    let (events_tx, events) = mpsc::channel();
    let (finish, finished) = mpsc::channel();
    let (peer, cancel, worker) = request(move |_| {
        events_tx.send(()).unwrap();
        finished.recv_timeout(WAIT).unwrap();
        false
    });
    peer.send(Duration::ZERO, HEADERS);
    peer.send(Duration::ZERO, b"data: done\n\n");
    events.recv_timeout(WAIT).unwrap();
    peer.send(Duration::ZERO, b"data: unfinished");
    peer.read();
    finish.send(()).unwrap();
    assert_eq!(worker.join().unwrap(), Ok(()));
    assert!(!cancel.load(Ordering::SeqCst));
    peer.replies.send(Some(b"x")).unwrap();
    peer.closed();
}

#[test]
fn a_shorter_ureq_deadline_keeps_its_reason() {
    let (peer, inner) = transport();
    let mut transport = CancelTransport {
        inner: Box::new(inner),
        cancel: Arc::new(AtomicBool::new(false)),
        stop: Arc::new(AtomicBool::new(false)),
        clock: peer.clock.clone(),
    };
    let worker = thread::spawn(move || {
        transport.await_input(NextTimeout {
            after: (SLICE / 2).into(),
            reason: ureq::Timeout::RecvBody,
        })
    });
    let timeout = peer.read();
    assert_eq!(*timeout.after, SLICE / 2);
    peer.advance(*timeout.after);
    peer.replies.send(None).unwrap();
    assert!(matches!(
        worker.join().unwrap(),
        Err(ureq::Error::Timeout(ureq::Timeout::RecvBody))
    ));
    peer.closed();
}
