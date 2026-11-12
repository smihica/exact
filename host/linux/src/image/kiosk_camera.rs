//! Raspberry Pi camera probe: one explicitly enabled stream, latest frame only.
//! This is not the cross-platform @exact/camera implementation.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use tiny_skia::Pixmap;

// KIOSK: analysis consumers share the preview stream instead of opening the device twice.
/// One unrotated, tightly packed luminance plane from the kiosk capture.
#[derive(Clone)]
pub struct LumaFrame {
    /// Plane width in pixels.
    pub width: usize,
    /// Plane height in pixels.
    pub height: usize,
    /// Row-major luminance samples, one byte per pixel.
    pub pixels: Arc<[u8]>,
}

type Reader = std::sync::mpsc::SyncSender<LumaFrame>;
static READERS: Mutex<Vec<Reader>> = Mutex::new(Vec::new());

/// Subscribe to the fixed kiosk camera's unrotated luminance frames.
/// The one-frame mailbox drops frames while the consumer is busy. Dropping the
/// receiver unsubscribes on the next frame; it never starts or owns the camera.
pub fn frames() -> std::sync::mpsc::Receiver<LumaFrame> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    READERS.lock().unwrap().push(sender);
    receiver
}

fn publish(readers: &mut Vec<Reader>, pixels: &[u8], width: usize, height: usize) {
    if readers.is_empty() {
        return;
    }
    let frame = LumaFrame {
        width,
        height,
        pixels: pixels.into(),
    };
    readers.retain(|reader| {
        !matches!(
            reader.try_send(frame.clone()),
            Err(std::sync::mpsc::TrySendError::Disconnected(_))
        )
    });
}

pub(super) const SOURCE: &str = "kiosk-camera:preview";
const WIDTH: usize = 640;
const HEIGHT: usize = 480;

enum Output {
    Frame(Pixmap),
    Finished(Result<(), String>),
}
type Mailbox = Arc<Mutex<Option<Output>>>;
struct Session {
    stop: Arc<AtomicBool>,
    latest: Mailbox,
    thread: JoinHandle<()>,
}

pub(super) struct Camera {
    pub enabled: bool,
    wanted: bool,
    failed: bool,
    session: Option<Session>,
}
impl Camera {
    pub fn new() -> Self {
        Self {
            enabled: std::env::var("KIOSK_CAMERA").as_deref() == Ok("1"),
            wanted: false,
            failed: false,
            session: None,
        }
    }
    pub fn request(&mut self, wanted: bool) {
        self.wanted = self.enabled && wanted;
        if !self.wanted {
            self.failed = false;
            if let Some(session) = &self.session {
                session.stop.store(true, Ordering::Relaxed);
            }
        }
    }
    pub fn poll(
        &mut self,
        backend: &Arc<super::workers::Backend>,
    ) -> Option<Result<Pixmap, String>> {
        let output = self
            .session
            .as_ref()
            .and_then(|s| s.latest.lock().unwrap().take());
        match output {
            Some(Output::Frame(frame)) => {
                if self.wanted && !self.session.as_ref().unwrap().stop.load(Ordering::Relaxed) {
                    return Some(Ok(frame));
                }
            }
            Some(Output::Finished(result)) => {
                let session = self.session.take().unwrap();
                let stopped = session.stop.load(Ordering::Relaxed);
                // Finished is published only after capture has killed/reaped its child.
                let _ = session.thread.join();
                if self.wanted && !stopped {
                    self.failed = true;
                    return Some(Err(result.err().unwrap_or_else(|| "Camera stopped".into())));
                }
            }
            None => {}
        }
        if self.wanted && !self.failed && self.session.is_none() {
            let stop = Arc::new(AtomicBool::new(false));
            let latest = Arc::new(Mutex::new(None));
            let backend = backend.clone();
            let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || backend.notify());
            let worker_stop = stop.clone();
            let worker_latest = latest.clone();
            match std::thread::Builder::new()
                .name("camera-preview".into())
                .spawn(move || {
                    let result = capture(worker_stop, worker_latest.clone(), wake.clone());
                    *worker_latest.lock().unwrap() = Some(Output::Finished(result));
                    wake();
                }) {
                Ok(thread) => {
                    self.session = Some(Session {
                        stop,
                        latest,
                        thread,
                    })
                }
                Err(error) => {
                    self.failed = true;
                    return Some(Err(error.to_string()));
                }
            }
        }
        None
    }
}
impl Drop for Camera {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            session.stop.store(true, Ordering::Relaxed);
            let _ = session.thread.join();
        }
    }
}

fn capture(
    stop: Arc<AtomicBool>,
    latest: Mailbox,
    wake: Arc<dyn Fn() + Send + Sync>,
) -> Result<(), String> {
    use std::{
        io::Read,
        os::fd::AsRawFd,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    // At this fixed resolution rpicam-vid emits tightly packed YUV420/sYCC.
    let mut child = Command::new("rpicam-vid")
        .args([
            "--nopreview",
            "--timeout",
            "0",
            "--width",
            "640",
            "--height",
            "480",
            "--framerate",
            "30",
            "--codec",
            "yuv420",
            "--flush",
            "--output",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("rpicam-vid: {error}"))?;
    let mut output = child.stdout.take().unwrap();
    let result = (|| {
        let mut bytes = vec![0; WIDTH * HEIGHT * 3 / 2];
        let mut used = 0;
        let mut last_frame = Instant::now();
        let mut frames = 0_u64;
        let mut sample = Instant::now();
        let mut conversion = Duration::ZERO;
        while !stop.load(Ordering::Relaxed) {
            if last_frame.elapsed() > Duration::from_secs(5) {
                return Err("No camera frame for 5 seconds".into());
            }
            let mut fd = libc::pollfd {
                fd: output.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // A single live pipe reader; cancellation remains bounded if capture stalls.
            #[allow(unsafe_code)]
            let ready = unsafe { libc::poll(&mut fd, 1, 50) };
            if ready < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error.to_string());
            }
            if ready == 0 {
                continue;
            }
            match output.read(&mut bytes[used..]) {
                Ok(0) => return Err("Camera stream ended".into()),
                Ok(size) => used += size,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.to_string()),
            }
            if used == bytes.len() {
                publish(
                    &mut READERS.lock().unwrap(),
                    &bytes[..WIDTH * HEIGHT],
                    WIDTH,
                    HEIGHT,
                );
                let started = Instant::now();
                let frame = portrait_frame(&bytes, WIDTH, HEIGHT);
                conversion += started.elapsed();
                // Slow rendering drops old frames instead of building a latency queue.
                *latest.lock().unwrap() = Some(Output::Frame(frame));
                wake();
                frames += 1;
                last_frame = Instant::now();
                used = 0;
                if frames.is_multiple_of(60) {
                    eprintln!(
                        "Camera frame={frames}; receive={:.1} fps; conversion={:.2} ms/frame",
                        60. / sample.elapsed().as_secs_f64(),
                        conversion.as_secs_f64() * 1000. / 60.
                    );
                    sample = Instant::now();
                    conversion = Duration::ZERO;
                }
            }
        }
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn portrait_frame(bytes: &[u8], width: usize, height: usize) -> Pixmap {
    let plane = width * height;
    let mut frame = Pixmap::new(height as u32, width as u32).unwrap();
    let pixels = frame.data_mut();
    for row in 0..height {
        for col in 0..width {
            let y = i32::from(bytes[row * width + col]);
            let uv = (row / 2) * (width / 2) + col / 2;
            let u = i32::from(bytes[plane + uv]) - 128;
            let v = i32::from(bytes[plane + plane / 4 + uv]) - 128;
            // Full-range sYCC; clockwise 90 degrees matches the physical camera mounting.
            let index = (col * height + height - 1 - row) * 4;
            pixels[index..index + 4].copy_from_slice(&[
                (y + ((22970 * v + 8192) >> 14)).clamp(0, 255) as u8,
                (y - ((5638 * u + 11700 * v + 8192) >> 14)).clamp(0, 255) as u8,
                (y + ((29032 * u + 8192) >> 14)).clamp(0, 255) as u8,
                255,
            ]);
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kiosk_camera_reader_is_bounded_and_unsubscribes() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let mut readers = vec![tx];
        publish(&mut readers, &[1, 2], 2, 1);
        publish(&mut readers, &[3, 4], 2, 1);
        let frame = rx.try_recv().unwrap();
        assert_eq!((frame.width, frame.height), (2, 1));
        assert_eq!(&*frame.pixels, &[1, 2]);
        assert!(rx.try_recv().is_err());
        publish(&mut readers, &[5, 6], 2, 1);
        assert_eq!(&*rx.try_recv().unwrap().pixels, &[5, 6]);
        drop(rx);
        publish(&mut readers, &[7, 8], 2, 1);
        assert!(readers.is_empty());
    }

    #[test]
    fn kiosk_camera_neutral_chroma_and_clockwise_rotation() {
        let image = portrait_frame(&[0, 255, 64, 128, 128, 128], 2, 2);
        let gray: Vec<u8> = image
            .data()
            .chunks_exact(4)
            .map(|p| {
                assert_eq!(p, &[p[0], p[0], p[0], 255]);
                p[0]
            })
            .collect();
        assert_eq!(gray, [64, 0, 128, 255]);
    }
    #[test]
    fn kiosk_camera_chroma_planes_rotate_with_the_image() {
        let image = portrait_frame(&[76, 76, 29, 29, 76, 76, 29, 29, 85, 255, 255, 107], 4, 2);
        assert_eq!((image.width(), image.height()), (2, 4));
        for (i, p) in image.data().chunks_exact(4).enumerate() {
            if i < 4 {
                assert!(p[0] >= 253 && p[1] <= 2 && p[2] <= 2);
            } else {
                assert!(p[2] >= 253 && p[0] <= 2 && p[1] <= 2);
            }
        }
    }
}
