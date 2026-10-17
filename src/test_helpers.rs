//! Shared test utilities across modules.
//!
//! Included via main.rs & available as `crate::test_helpers`.

use std::io::{self, Write};

use crate::adapters::command::CommandRunner;
use crate::domain::error::Error;

/// A `Write` implementation that always fails on `write` but succeeds on `flush`.
pub(crate) struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("fail"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A `Write` implementation that fails on both `write` and `flush`.
pub(crate) struct FailingFlushWriter;

impl Write for FailingFlushWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("fail"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fail"))
    }
}

/// A `Write` implementation where `write` succeeds but `flush` fails.
pub(crate) struct FlushFailingWriter;

impl Write for FlushFailingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fail"))
    }
}

type RunFn = Box<dyn Fn(&[String]) -> Result<i32, Error>>;
type CaptureFn = Box<dyn Fn(&[String]) -> Result<(i32, String), Error>>;
type ProbeFn = Box<dyn Fn(&[String]) -> Result<i32, Error>>;

/// A [`CommandRunner`] whose operations can be overridden per test; the
/// defaults succeed.
pub(crate) struct StubRunner {
    run: RunFn,
    capture: CaptureFn,
    probe: ProbeFn,
}

impl StubRunner {
    pub(crate) fn new() -> Self {
        Self {
            run: Box::new(no_run),
            capture: Box::new(no_capture),
            probe: Box::new(no_probe),
        }
    }

    pub(crate) fn with_run(
        mut self,
        f: impl Fn(&[String]) -> Result<i32, Error> + 'static,
    ) -> Self {
        self.run = Box::new(f);
        self
    }

    pub(crate) fn with_capture(
        mut self,
        f: impl Fn(&[String]) -> Result<(i32, String), Error> + 'static,
    ) -> Self {
        self.capture = Box::new(f);
        self
    }

    pub(crate) fn with_probe(
        mut self,
        f: impl Fn(&[String]) -> Result<i32, Error> + 'static,
    ) -> Self {
        self.probe = Box::new(f);
        self
    }
}

impl CommandRunner for StubRunner {
    fn run(&self, cmd: &[String]) -> Result<i32, Error> {
        (self.run)(cmd)
    }
    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error> {
        (self.capture)(cmd)
    }
    fn probe(&self, cmd: &[String]) -> Result<i32, Error> {
        (self.probe)(cmd)
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "stub must match the CommandRunner trait signature"
)]
fn no_run(_: &[String]) -> Result<i32, Error> {
    Ok(0)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "stub must match the CommandRunner trait signature"
)]
fn no_capture(_: &[String]) -> Result<(i32, String), Error> {
    Ok((0, String::new()))
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "stub must match the CommandRunner trait signature"
)]
fn no_probe(_: &[String]) -> Result<i32, Error> {
    Ok(0)
}
