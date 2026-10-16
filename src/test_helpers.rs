//! Shared test utilities across modules.
//!
//! Included via main.rs & available as `crate::test_helpers`.

use std::io::{self, Write};

/// A `Write` implementation that always fails on `write` but succeeds on `flush`.
pub struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("fail"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A `Write` implementation that fails on both `write` and `flush`.
pub struct FailingFlushWriter;

impl Write for FailingFlushWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("fail"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fail"))
    }
}

/// A `Write` implementation where `write` succeeds but `flush` fails.
pub struct FlushFailingWriter;

impl Write for FlushFailingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fail"))
    }
}
