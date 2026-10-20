use std::io::{self, BufRead, Write};

use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::shared::ansi::{self, Palette};
use crate::system_update::ports::user_io::UserIo;

/// Production [`UserIo`]: prompts and status on stdout, diagnostics on stderr.
pub(crate) struct TerminalIo {
    palette: Palette,
}

impl TerminalIo {
    pub(crate) fn new(palette: Palette) -> Self {
        Self { palette }
    }
}

impl UserIo for TerminalIo {
    fn title(&self, text: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        print_title(&mut out, text, self.palette)
    }

    fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut out = io::stdout();
        confirm(&mut reader, &mut out, default, msg, self.palette)
    }

    fn error(&self, msg: &str) -> Result<(), Error> {
        // Diagnostics go to stderr so they survive stdout being piped.
        let mut err = io::stderr();
        write_line(
            &mut err,
            &format!("{}{}{}", self.palette.red, msg, self.palette.reset),
        )
    }

    fn success(&self, msg: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        write_line(
            &mut out,
            &format!("{}{}{}", self.palette.green, msg, self.palette.reset),
        )
    }

    fn output(&self, text: &str) -> Result<(), Error> {
        let mut out = io::stdout();
        write_raw(&mut out, text)
    }

    fn config(&self, config: &Config) -> Result<(), Error> {
        let mut out = io::stdout();
        print_config(&mut out, config, self.palette)
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

/// Print a banner title with colored borders.
///
/// Note: `text.len()` counts bytes, not graphemes. Non-ASCII text
/// will misalign borders. Acceptable for ASCII-only CLI output.
pub(crate) fn print_title(
    writer: &mut dyn Write,
    text: &str,
    palette: Palette,
) -> Result<(), Error> {
    let border: String = "*".repeat(text.len() + 4);
    ansi::write_flush(
        writer,
        &format!(
            "{}\n{}\n* {}{}{} *\n{}\n{}",
            palette.blue, border, palette.red, text, palette.blue, border, palette.reset
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
    palette: Palette,
) -> Result<bool, Error> {
    write_confirm_prompt(writer, default, msg, palette)?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(parse_confirm_response(line.trim(), default))
}

fn write_confirm_prompt(
    writer: &mut dyn Write,
    default: bool,
    msg: Option<&str>,
    palette: Palette,
) -> Result<(), Error> {
    let hint = if default {
        format!("{}(Y/n){}", palette.green, palette.reset)
    } else {
        format!("{}(y/N){}", palette.red, palette.reset)
    };
    let text = match msg {
        Some(value) => format!("\n{}{}{} {}: ", palette.yellow, value, palette.reset, hint),
        None => format!("\n{}Proceed?{} {}: ", palette.yellow, palette.reset, hint),
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

/// Print all `config` fields with colored labels, each terminated by a newline.
pub(crate) fn print_config(
    writer: &mut dyn Write,
    config: &Config,
    palette: Palette,
) -> Result<(), Error> {
    print_config_line(writer, "repo", &config.repo, palette)?;
    print_config_line(writer, "hostname", &config.hostname, palette)?;
    print_config_line(writer, "keep", &config.keep.to_string(), palette)?;
    print_config_line(writer, "update", &config.update.to_string(), palette)?;
    print_config_line(writer, "diff", &config.diff.to_string(), palette)?;
    // Shown only when set, so a dry run is never mistaken for a real one.
    if config.dry_run {
        print_config_line(writer, "dry-run", &config.dry_run.to_string(), palette)?;
    }
    Ok(())
}

fn print_config_line(
    writer: &mut dyn Write,
    label: &str,
    value: &str,
    palette: Palette,
) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        &format!(
            "{}◉ {}{}{} = {}{}{}\n",
            palette.cyan, palette.red, label, palette.reset, palette.cyan, value, palette.reset
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
