use std::io::{self, BufRead, Write};

use crate::ansi;
use crate::config::Config;
use crate::error::Error;
use crate::ports::user_io::UserIo;

/// Production [`UserIo`] backed by stdin/stdout.
pub(crate) struct TerminalIo;

impl UserIo for TerminalIo {
    fn title(&self, text: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        print_title(&mut out, text)
    }

    fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut out = io::stdout();
        confirm(&mut reader, &mut out, default, msg)
    }

    fn error(&self, msg: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        write_line(&mut out, &format!("{}{}{}", ansi::RED, msg, ansi::RESET))
    }

    fn success(&self, msg: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        write_line(&mut out, &format!("{}{}{}", ansi::GREEN, msg, ansi::RESET))
    }

    fn output(&self, text: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        write_raw(&mut out, text)
    }

    fn config(&self, config: &Config) -> Result<(), Error> {
        let mut out = io::stdout();
        print_config(&mut out, config)
    }
}

fn write_line(writer: &mut dyn Write, text: &str) -> Result<(), Error> {
    ansi::write_flush(writer, &format!("{text}\n"))?;
    Ok(())
}

fn write_raw(writer: &mut dyn Write, text: &str) -> Result<(), Error> {
    ansi::write_flush(writer, text)?;
    Ok(())
}

/// Print a banner title with ANSI-colored borders.
///
/// Note: `text.len()` counts bytes, not graphemes. Non-ASCII text
/// will misalign borders. Acceptable for ASCII-only CLI output.
pub(crate) fn print_title(writer: &mut dyn Write, text: &str) -> Result<(), Error> {
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
pub(crate) fn confirm(
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
        None => format!("\n{}Proceed?{} {}: ", ansi::YELLOW, ansi::RESET, hint),
    };
    ansi::write_flush(writer, &text)?;
    Ok(())
}

/// Parse a confirmation response line.
///
/// Returns `default` for empty input, `true` for "y"/"yes", `false` for
/// "n"/"no" or any other input.
pub(crate) fn parse_confirm_response(line: &str, default: bool) -> bool {
    match line.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "" => default,
        _ => false,
    }
}

/// Print all `config` fields with ANSI styling, each terminated by a newline.
pub(crate) fn print_config(writer: &mut dyn Write, config: &Config) -> Result<(), Error> {
    print_config_line(writer, "repo", &config.repo)?;
    print_config_line(writer, "hostname", &config.hostname)?;
    print_config_line(writer, "keep", &config.keep.to_string())?;
    print_config_line(writer, "update", &config.update.to_string())?;
    print_config_line(writer, "diff", &config.diff.to_string())?;
    Ok(())
}

fn print_config_line(writer: &mut dyn Write, label: &str, value: &str) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        &format!(
            "{}◉ {}{}{} = {}{}{}\n",
            ansi::CYAN,
            ansi::RED,
            label,
            ansi::RESET,
            ansi::CYAN,
            value,
            ansi::RESET
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
