//! Progress reporting, inverted.
//!
//! The engine calls a [`Reporter`] instead of touching the terminal. Each
//! front-end supplies its own implementation:
//! - the CLI renders steps as `spinners` lines with themed symbols,
//! - CI prints plain, timestamped lines,
//! - the server-side git receiver streams lines over the push sideband.

/// How the engine tells the outside world what it is doing.
///
/// A "step" is one unit of work (detect runner, build, upload, restart). The
/// engine brackets each with [`step_start`](Reporter::step_start) and one of
/// [`step_done`](Reporter::step_done) / [`step_fail`](Reporter::step_fail).
pub trait Reporter: Send + Sync {
    /// A unit of work has started.
    fn step_start(&self, msg: &str);

    /// The current step finished successfully.
    fn step_done(&self, msg: &str);

    /// The current step failed.
    fn step_fail(&self, msg: &str);

    /// An informational note not tied to a step. Defaults to no-op.
    fn info(&self, msg: &str) {
        let _ = msg;
    }

    /// A single line of streamed output from a build or the remote (e.g. a line
    /// of `next build` output, or a git receive-pack message). Defaults to no-op.
    fn remote_line(&self, line: &str) {
        let _ = line;
    }
}

/// A [`Reporter`] that discards everything.
///
/// Useful for tests and for non-interactive callers that only care about the
/// final `Result`.
pub struct NoopReporter;

impl Reporter for NoopReporter {
    fn step_start(&self, _msg: &str) {}
    fn step_done(&self, _msg: &str) {}
    fn step_fail(&self, _msg: &str) {}
}
