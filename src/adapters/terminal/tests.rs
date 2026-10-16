
use super::*;
use std::io::{self, Read};

fn test_config() -> Config {
    Config {
        repo: String::from("r"),
        hostname: String::from("h"),
        keep: 1,
        update: false,
        diff: false,
        yes: false,
        dry_run: false,
        elevate: false,
    }
}

// ------------------------------------------------------------------
// print_title / confirm / parse_confirm_response
// ------------------------------------------------------------------

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

// ------------------------------------------------------------------
// print_config
// ------------------------------------------------------------------

#[test]
fn print_config_renders_all_fields() {
    let mut buf = Vec::new();
    let config = Config {
        repo: String::from("~/.dotfiles"),
        hostname: String::from("nixos"),
        keep: 10,
        update: false,
        diff: true,
        yes: false,
        dry_run: false,
        elevate: true,
    };
    assert!(print_config(&mut buf, &config).is_ok());
    let s = String::from_utf8_lossy(&buf);
    assert!(s.contains("repo"));
    assert!(s.contains("hostname"));
    assert!(s.contains("keep"));
    assert!(s.contains("update"));
    assert!(s.contains("diff"));
}

#[test]
fn print_config_last_field_has_no_trailing_newline() {
    let mut buf = Vec::new();
    let config = test_config();
    assert!(print_config(&mut buf, &config).is_ok());
    let s = String::from_utf8_lossy(&buf);
    assert!(!s.ends_with('\n'));
}

struct CountingWriter {
    fail_after: usize,
    count: usize,
}

impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.count += 1;
        if self.count >= self.fail_after {
            Err(io::Error::other("fail"))
        } else {
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn print_config_error_propagation_line_1() {
    let mut writer = CountingWriter {
        fail_after: 1,
        count: 0,
    };
    assert!(print_config(&mut writer, &test_config()).is_err());
}

#[test]
fn print_config_error_propagation_line_2() {
    let mut writer = CountingWriter {
        fail_after: 2,
        count: 0,
    };
    assert!(print_config(&mut writer, &test_config()).is_err());
}

#[test]
fn print_config_error_propagation_line_3() {
    let mut writer = CountingWriter {
        fail_after: 3,
        count: 0,
    };
    assert!(print_config(&mut writer, &test_config()).is_err());
}

#[test]
fn print_config_error_propagation_line_4() {
    let mut writer = CountingWriter {
        fail_after: 4,
        count: 0,
    };
    assert!(print_config(&mut writer, &test_config()).is_err());
}

#[test]
fn print_config_error_propagation_line_5() {
    let mut writer = CountingWriter {
        fail_after: 5,
        count: 0,
    };
    assert!(print_config(&mut writer, &test_config()).is_err());
}

#[test]
fn print_config_line_error_propagation() {
    let mut writer = crate::test_helpers::FailingWriter;
    assert!(print_config_line(&mut writer, "k", "v", true).is_err());
}

// ------------------------------------------------------------------
// TerminalIo — thin glue layer
// ------------------------------------------------------------------

#[test]
fn terminal_io_title_does_not_panic() {
    TerminalIo.title("Test").expect("title");
}

#[test]
fn terminal_io_error_does_not_panic() {
    TerminalIo.error("err").expect("error");
}

#[test]
fn terminal_io_success_does_not_panic() {
    TerminalIo.success("ok").expect("success");
}

#[test]
fn terminal_io_config_does_not_panic() {
    TerminalIo.config(&test_config()).expect("config");
}

#[test]
fn terminal_io_output_does_not_panic() {
    TerminalIo.output("raw\n").expect("output");
}

#[test]
fn write_line_writes_text_with_newline() {
    let mut buf = Vec::new();
    assert!(write_line(&mut buf, "plain").is_ok());
    assert_eq!(buf, b"plain\n");
}

#[test]
fn write_line_error_propagation() {
    let mut writer = crate::test_helpers::FailingWriter;
    assert!(write_line(&mut writer, "plain").is_err());
}

#[test]
fn write_raw_writes_text_verbatim() {
    let mut buf = Vec::new();
    assert!(write_raw(&mut buf, "raw").is_ok());
    assert_eq!(buf, b"raw");
}

#[test]
fn write_raw_error_propagation() {
    let mut writer = crate::test_helpers::FailingWriter;
    assert!(write_raw(&mut writer, "raw").is_err());
}
