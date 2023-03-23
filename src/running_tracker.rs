use std::sync::{atomic, Arc};

use once_cell::sync::Lazy;
use tracing::info;

// pub static RUNNING_TRACKER: Lazy<Arc<tokio::sync::Notify>> =
//     Lazy::new(|| Arc::new(tokio::sync::Notify::new()));

pub static RUNNING_TRACKER: Lazy<Arc<RunningTracker>> =
    Lazy::new(|| Arc::new(RunningTracker::new()));

pub struct RunningTracker {
    notify: tokio::sync::Notify,
    exit_code: atomic::AtomicI32,
}

impl RunningTracker {
    fn new() -> Self {
        RunningTracker {
            notify: tokio::sync::Notify::new(),
            exit_code: atomic::AtomicI32::new(0),
        }
    }

    pub fn quit(&self, reason: &str) {
        self.notify.notify_waiters();
        info!("Quit {}", reason);
    }

    pub fn quit_with_code(&self, code: i32, reason: &str) {
        self.notify.notify_waiters();
        self.exit_code.store(code, atomic::Ordering::Relaxed);
        info!("Quit with code {}: {}", code, reason);
    }

    #[allow(unused)]
    pub fn exit_code(&self) -> i32 {
        self.exit_code.load(atomic::Ordering::Relaxed)
    }

    pub async fn wait_quit(&self) {
        self.notify.notified().await
    }
}
