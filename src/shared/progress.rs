//! OSC 9;4 progress reports: the bar a terminal shows in the tab while a
//! program works.
//!
//! OSC 7501 tells a supporting terminal *what* `rx` is doing; this tells it
//! *that* something is running. The bar uses the protocol's indeterminate
//! state, which the terminal animates as a pulse; it is re-emitted ten times a
//! second because some terminals only advance (and keep) the animation when a
//! fresh report arrives. The bar must vanish whenever the user has to
//! intervene.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::shared::ansi;

/// Indeterminate progress: state `3`, no percentage to give.
const RUNNING: &str = "\x1b]9;4;3;0\x1b\\";
/// Remove the bar: state `0`.
const CLEAR: &str = "\x1b]9;4;0;0\x1b\\";
/// Ten reports a second: enough for the terminal's pulse and for it never to
/// consider the report stale.
const INTERVAL: Duration = Duration::from_millis(100);

/// The tab progress bar of one run.
///
/// Every write is best-effort: a closed stdout must never fail the update, and
/// a terminal that ignores OSC 9;4 simply shows nothing.
pub(crate) struct Progress {
    enabled: bool,
    active: AtomicBool,
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Progress {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            active: AtomicBool::new(false),
            stop: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }

    /// Show the bar and keep its animation alive.
    pub(crate) fn show(&self) {
        if !self.enabled || self.active.swap(true, Ordering::AcqRel) {
            return;
        }
        self.stop.store(false, Ordering::Release);
        emit(RUNNING);
        let stop = Arc::clone(&self.stop);
        let handle = thread::spawn(move || keep_alive(INTERVAL, &stop, usize::MAX));
        *lock(&self.thread) = Some(handle);
    }

    /// Hide the bar: the user has to intervene, or the run is over.
    pub(crate) fn hide(&self) {
        if !self.enabled || !self.active.swap(false, Ordering::AcqRel) {
            return;
        }
        self.stop.store(true, Ordering::Release);
        let handle = lock(&self.thread)
            .take()
            .expect("an active progress bar owns its thread");
        handle.thread().unpark();
        // Best effort: the bar is going away, a failed join changes nothing.
        let _ = handle.join();
        emit(CLEAR);
    }
}

/// Lock the thread slot; the thread never panics while holding it.
fn lock(slot: &Mutex<Option<JoinHandle<()>>>) -> std::sync::MutexGuard<'_, Option<JoinHandle<()>>> {
    slot.lock().expect("progress thread slot is never poisoned")
}

/// Re-emit the bar while it is up.
///
/// `writes_left` bounds the loop so a test can run it without a thread;
/// production uses `usize::MAX` and relies on `stop`.
fn keep_alive(interval: Duration, stop: &AtomicBool, writes_left: usize) {
    let mut writes = 0;
    while writes < writes_left && refresh(stop) {
        writes += 1;
        thread::park_timeout(interval);
    }
}

/// Whether the bar is still up; re-emits it when it is.
fn refresh(stop: &AtomicBool) -> bool {
    if stop.load(Ordering::Acquire) {
        return false;
    }
    emit(RUNNING);
    true
}

/// Write a sequence to stdout, best-effort.
fn emit(sequence: &str) {
    let mut out = io::stdout();
    // Best effort: decorative, a dead terminal must not fail the update.
    let _ = ansi::write_flush(&mut out, sequence);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_sequence_is_indeterminate_progress() {
        assert_eq!(RUNNING, "\x1b]9;4;3;0\x1b\\");
    }

    #[test]
    fn clear_sequence_removes_the_bar() {
        assert_eq!(CLEAR, "\x1b]9;4;0;0\x1b\\");
    }

    #[test]
    fn show_and_hide_drive_the_bar() {
        let bar = Progress::new(true);
        bar.hide(); // nothing to hide yet
        bar.show();
        bar.show(); // already up
        bar.hide();
        bar.hide(); // already down
    }

    #[test]
    fn disabled_bar_ignores_every_call() {
        let bar = Progress::new(false);
        bar.show();
        bar.hide();
    }

    #[test]
    fn refresh_stops_on_the_stop_flag() {
        assert!(!refresh(&AtomicBool::new(true)));
    }

    #[test]
    fn refresh_reemits_while_the_bar_is_up() {
        assert!(refresh(&AtomicBool::new(false)));
    }

    #[test]
    fn keep_alive_honours_its_write_budget() {
        keep_alive(Duration::ZERO, &AtomicBool::new(false), 1);
    }

    #[test]
    fn keep_alive_stops_when_asked() {
        keep_alive(Duration::ZERO, &AtomicBool::new(true), 1);
    }
}
