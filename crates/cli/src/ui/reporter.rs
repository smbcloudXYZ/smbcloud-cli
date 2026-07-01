//! The CLI's [`Reporter`] implementation.
//!
//! This is the one place the deploy engine's progress turns into terminal
//! output. The engine calls `step_start` / `step_done` / `step_fail`; here we
//! render them as `spinners` lines with the shared themed symbols. CI and the
//! server-side receiver supply their own `Reporter` instead.

use crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol};
use smbcloud_deploy::Reporter;
use spinners::{Spinner, Spinners};
use std::sync::Mutex;

/// Renders engine progress as spinner lines. One spinner is live at a time; a
/// `step_*` completion persists it with a symbol, and the next `step_start`
/// begins a fresh one.
pub struct SpinnerReporter {
    current: Mutex<Option<Spinner>>,
}

impl SpinnerReporter {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    /// Persist the active spinner (if any) with `symbol` + `message`, or print a
    /// standalone line when no spinner is running.
    fn finish(&self, symbol: String, message: String) {
        match self.current.lock().unwrap().take() {
            Some(mut spinner) => spinner.stop_and_persist(&symbol, message),
            None => println!("{symbol} {message}"),
        }
    }
}

impl Default for SpinnerReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Reporter for SpinnerReporter {
    fn step_start(&self, msg: &str) {
        // Starting a new step supersedes any spinner left running.
        *self.current.lock().unwrap() =
            Some(Spinner::new(Spinners::SimpleDotsScrolling, succeed_message(msg)));
    }

    fn step_done(&self, msg: &str) {
        self.finish(succeed_symbol(), succeed_message(msg));
    }

    fn step_fail(&self, msg: &str) {
        self.finish(fail_symbol(), fail_message(msg));
    }

    fn info(&self, msg: &str) {
        println!("{}", succeed_message(msg));
    }

    fn remote_line(&self, line: &str) {
        println!("{line}");
    }
}
