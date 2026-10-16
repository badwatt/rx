use std::io::{BufRead, Write};

use crate::core::ansi;
use crate::error::Error;

/// Print a banner title with ANSI-colored borders.
///
/// Note: `text.len()` counts bytes, not graphemes. Non-ASCII text
/// will misalign borders. Acceptable for ASCII-only CLI output.
pub fn print_title(writer: &mut dyn Write, text: &str) -> Result<(), Error> {
    let border: String = "*".repeat(text.len() + 4);
    ansi::write_flush(
        writer,
        &format!(
            "{}\n{}\n* {}{}{} *\n{}\n{}",
            ansi::BLUE,
            border,
            ansi::RED,
            text,
            ansi::BLUE,
            border,
            ansi::RESET
        ),
    )?;
    Ok(())
}

/// Prompt user for confirmation on `reader`/`writer`.
///
/// `default` is returned when the user sends an empty line.
pub fn confirm(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    default: bool,
    msg: Option<&str>,
) -> Result<bool, Error> {
    write_confirm_prompt(writer, default, msg)?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(parse_confirm_response(line.trim(), default))
}

fn write_confirm_prompt(
    writer: &mut dyn Write,
    default: bool,
    msg: Option<&str>,
) -> Result<(), Error> {
    let hint = if default {
        format!("{}(Y/n){}", ansi::GREEN, ansi::RESET)
    } else {
        format!("{}(y/N){}", ansi::RED, ansi::RESET)
    };
    let text = match msg {
        Some(value) => format!("\n{}{}{} {}: ", ansi::YELLOW, value, ansi::RESET, hint),
        None => format!("\n\n{}Proceed?{} {}: ", ansi::YELLOW, ansi::RESET, hint),
    };
    ansi::write_flush(writer, &text)?;
    Ok(())
}

/// Parse a confirmation response line.
///
/// Returns `default` for empty input, `true` for "y"/"yes", `false` for
/// "n"/"no" or any other input.
pub fn parse_confirm_response(line: &str, default: bool) -> bool {
    match line.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "" => default,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read};

    #[test]
    fn print_title_contains_text_and_ansi_codes() {
        let mut buf = Vec::new();
        assert!(print_title(&mut buf, "RX").is_ok());
        let s = String::from_utf8_lossy(&buf);
        assert!(s.contains("RX"));
        assert!(s.contains(ansi::BLUE));
        assert!(s.contains(ansi::RED));
        assert!(s.contains(ansi::RESET));
    }

    #[test]
    fn parse_confirm_y_returns_true() {
        assert!(parse_confirm_response("y", false));
    }

    #[test]
    fn parse_confirm_uppercase_y_returns_true() {
        assert!(parse_confirm_response("Y", false));
    }

    #[test]
    fn parse_confirm_yes_returns_true() {
        assert!(parse_confirm_response("yes", false));
    }

    #[test]
    fn parse_confirm_n_returns_false() {
        assert!(!parse_confirm_response("n", true));
    }

    #[test]
    fn parse_confirm_no_returns_false() {
        assert!(!parse_confirm_response("no", true));
    }

    #[test]
    fn parse_confirm_empty_with_true_default_returns_true() {
        assert!(parse_confirm_response("", true));
    }

    #[test]
    fn parse_confirm_empty_with_false_default_returns_false() {
        assert!(!parse_confirm_response("", false));
    }

    #[test]
    fn parse_confirm_garbage_returns_false() {
        assert!(!parse_confirm_response("xyz", true));
    }

    #[test]
    fn confirm_reads_y_and_returns_true() {
        let mut reader = io::BufReader::new(b"y\n".as_slice());
        let mut writer = Vec::new();
        assert!(confirm(&mut reader, &mut writer, false, None).expect("confirm"));
    }

    #[test]
    fn confirm_reads_n_and_returns_false() {
        let mut reader = io::BufReader::new(b"n\n".as_slice());
        let mut writer = Vec::new();
        assert!(!confirm(&mut reader, &mut writer, true, None).expect("confirm"));
    }

    #[test]
    fn confirm_with_message_writes_message_to_writer() {
        let mut reader = io::BufReader::new(b"y\n".as_slice());
        let mut writer = Vec::new();
        assert!(confirm(&mut reader, &mut writer, false, Some("Sure")).is_ok());
        let output = String::from_utf8_lossy(&writer);
        assert!(output.contains("Sure"));
    }

    #[test]
    fn confirm_empty_line_uses_default_true() {
        let mut reader = io::BufReader::new(b"\n".as_slice());
        let mut writer = Vec::new();
        assert!(confirm(&mut reader, &mut writer, true, None).expect("confirm"));
    }

    #[test]
    fn failing_writer_flush_is_ok() {
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(io::Write::flush(&mut writer).is_ok());
    }

    struct FailingReader;

    impl io::Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(std::io::Error::other("fail"))
        }
    }

    impl BufRead for FailingReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(std::io::Error::other("fail"))
        }
        fn consume(&mut self, _: usize) {}
    }

    #[test]
    fn failing_reader_read_is_err() {
        let mut reader = FailingReader;
        let mut buf = [0u8; 1];
        assert!(reader.read(&mut buf).is_err());
    }

    #[test]
    fn failing_reader_consume_is_noop() {
        let mut reader = FailingReader;
        reader.consume(0);
    }

    #[test]
    fn print_title_error_propagation() {
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(print_title(&mut writer, "Test").is_err());
    }

    #[test]
    fn confirm_error_from_writer() {
        let mut reader = io::BufReader::new(b"y\n".as_slice());
        let mut writer = crate::test_helpers::FailingWriter;
        let result = confirm(&mut reader, &mut writer, false, None);
        assert!(result.is_err());
    }

    #[test]
    fn confirm_error_from_writer_with_message() {
        let mut reader = io::BufReader::new(b"y\n".as_slice());
        let mut writer = crate::test_helpers::FailingWriter;
        let result = confirm(&mut reader, &mut writer, false, Some("Sure"));
        assert!(result.is_err());
    }

    #[test]
    fn confirm_error_from_reader_read_line() {
        let mut reader = FailingReader;
        let mut writer = Vec::new();
        let result = confirm(&mut reader, &mut writer, true, None);
        assert!(result.is_err());
    }
}
