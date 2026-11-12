//! A kept page's best variants (brotli at 11, gzip at 9), made off every
//! request's path by one thread for the whole server, as the dist's files
//! are. A page replaced before its turn is skipped: its `OnceLock` went with
//! it, so its job's weak reference no longer upgrades. With a thread for each
//! page, reloads (`Cache-Control: max-age=0`) piled compressions up without
//! bound.

use crate::encode::Best;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

struct Job {
    best: Weak<OnceLock<Best>>,
    body: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct Compressor {
    jobs: Mutex<VecDeque<Job>>,
    ready: Condvar,
    worker: OnceLock<()>,
}

impl Compressor {
    /// Make `body`'s best variants into `best` when the worker reaches it,
    /// unless the page holding `best` is gone by then.
    pub(crate) fn make(self: &Arc<Self>, best: &Arc<OnceLock<Best>>, body: Vec<u8>) {
        {
            let mut jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
            // A replaced page's job holds its body for nothing.
            jobs.retain(|job| job.best.strong_count() > 0);
            jobs.push_back(Job {
                best: Arc::downgrade(best),
                body,
            });
        }
        self.ready.notify_one();
        self.worker.get_or_init(|| {
            let queue = Arc::clone(self);
            let _ = std::thread::Builder::new()
                .name("exact-render-best".into())
                .spawn(move || queue.run());
        });
    }

    fn run(&self) {
        loop {
            let job = {
                let mut jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
                loop {
                    match jobs.pop_front() {
                        Some(job) => break job,
                        None => jobs = self.ready.wait(jobs).unwrap_or_else(|e| e.into_inner()),
                    }
                }
            };
            if let Some(best) = job.best.upgrade() {
                let _ = best.set(Best::of(&job.body));
            }
        }
    }

    /// Jobs waiting, for the tests.
    #[cfg(test)]
    pub(crate) fn waiting(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One worker makes every page's variants, and a page replaced before
    /// its turn costs nothing.
    #[test]
    fn a_replaced_page_is_skipped_and_a_kept_one_is_made() {
        let queue = Arc::new(Compressor::default());
        let body = "a page ".repeat(20_000).into_bytes();
        let kept = Arc::new(OnceLock::new());
        for _ in 0..50 {
            let replaced = Arc::new(OnceLock::new());
            queue.make(&replaced, body.clone());
        }
        queue.make(&kept, body.clone());
        assert!(queue.waiting() <= 2, "{} jobs held", queue.waiting());
        let started = std::time::Instant::now();
        while kept.get().is_none() {
            assert!(started.elapsed().as_secs() < 30, "never made");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(kept.get().unwrap().bytes() > 0);
    }
}
