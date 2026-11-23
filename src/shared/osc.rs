//! OSC 7501 (Program Status Protocol) reports.
//!
//! `rx` tells a supporting terminal what the update is doing. Values use only
//! the alphabet the protocol allows, so free text in `msg` travels as base64
//! of its UTF-8 bytes; the encoder is hand-rolled to keep the crate
//! dependency-free.

use std::io::Write;

use crate::domain::error::Error;
use crate::shared::ansi;

/// The program states `rx` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Working,
    BlockedPermission,
    /// Blocked on a credential: sudo is asking for a password.
    BlockedAuth,
    Done,
    Failed,
}

impl State {
    /// Name the protocol uses for this state.
    fn name(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::BlockedPermission | Self::BlockedAuth => "blocked",
            Self::Done => "done",
            Self::Failed => "error",
        }
    }

    /// Why the program is blocked; only `blocked` carries a `kind`.
    fn kind(self) -> Option<&'static str> {
        match self {
            Self::BlockedPermission => Some("permission"),
            Self::BlockedAuth => Some("auth"),
            Self::Working | Self::Done | Self::Failed => None,
        }
    }
}

/// Write one `OSC 7501` report for `state`, with `msg` base64-encoded.
pub(crate) fn report(writer: &mut dyn Write, state: State, msg: &str) -> Result<(), Error> {
    let kind = match state.kind() {
        Some(kind) => format!(":kind={kind}"),
        None => String::new(),
    };
    let sequence = format!(
        "\x1b]7501;state={}{}:app=rx:msg={}\x1b\\",
        state.name(),
        kind,
        base64(msg)
    );
    ansi::write_flush(writer, &sequence)?;
    Ok(())
}

/// Standard base64 (RFC 4648, padded) of the UTF-8 bytes of `input`.
fn base64(input: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut group = [0u8; 3];
        group[..chunk.len()].copy_from_slice(chunk);
        let value = (u32::from(group[0]) << 16) | (u32::from(group[1]) << 8) | u32::from(group[2]);
        // A full group yields four characters; a 1- or 2-byte tail is padded.
        for slot in 0..4 {
            let symbol = if slot < chunk.len() + 1 {
                ALPHABET[((value >> (18 - 6 * slot)) & 0x3f) as usize]
            } else {
                b'='
            };
            out.push(char::from(symbol));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::test_helpers::FailingWriter;

    #[test]
    fn report_working_writes_the_exact_sequence() {
        let mut buf = Vec::new();
        assert!(report(&mut buf, State::Working, "Working").is_ok());
        assert_eq!(
            buf,
            b"\x1b]7501;state=working:app=rx:msg=V29ya2luZw==\x1b\\"
        );
    }

    #[test]
    fn report_blocked_permission_adds_the_kind() {
        let mut buf = Vec::new();
        assert!(report(&mut buf, State::BlockedPermission, "").is_ok());
        assert_eq!(
            buf,
            b"\x1b]7501;state=blocked:kind=permission:app=rx:msg=\x1b\\"
        );
    }

    #[test]
    fn report_blocked_auth_adds_the_auth_kind() {
        let mut buf = Vec::new();
        assert!(report(&mut buf, State::BlockedAuth, "").is_ok());
        assert_eq!(buf, b"\x1b]7501;state=blocked:kind=auth:app=rx:msg=\x1b\\");
    }

    #[test]
    fn report_done_writes_the_exact_sequence() {
        let mut buf = Vec::new();
        assert!(report(&mut buf, State::Done, "ok").is_ok());
        assert_eq!(buf, b"\x1b]7501;state=done:app=rx:msg=b2s=\x1b\\");
    }

    #[test]
    fn report_failed_uses_the_error_state_name() {
        let mut buf = Vec::new();
        assert!(report(&mut buf, State::Failed, "boom").is_ok());
        assert_eq!(buf, b"\x1b]7501;state=error:app=rx:msg=Ym9vbQ==\x1b\\");
    }

    #[test]
    fn report_encodes_utf8_text() {
        let mut buf = Vec::new();
        assert!(report(&mut buf, State::Done, "ñ€").is_ok());
        assert_eq!(buf, b"\x1b]7501;state=done:app=rx:msg=w7Higqw=\x1b\\");
    }

    #[test]
    fn report_propagates_a_write_error() {
        assert!(report(&mut FailingWriter, State::Working, "x").is_err());
    }

    #[test]
    fn base64_of_empty_input_is_empty() {
        assert_eq!(base64(""), "");
    }

    #[test]
    fn base64_pads_a_one_byte_tail() {
        assert_eq!(base64("f"), "Zg==");
    }

    #[test]
    fn base64_pads_a_two_byte_tail() {
        assert_eq!(base64("fo"), "Zm8=");
    }

    #[test]
    fn base64_of_three_bytes_has_no_padding() {
        assert_eq!(base64("foo"), "Zm9v");
    }

    #[test]
    fn base64_of_multi_group_input() {
        assert_eq!(base64("foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_of_utf8_text_encodes_the_utf8_bytes() {
        assert_eq!(base64("ñ€"), "w7Higqw=");
    }
}
