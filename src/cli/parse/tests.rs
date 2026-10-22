use super::*;

fn args(from: &[&str]) -> Vec<String> {
    from.iter().copied().map(str::to_string).collect()
}

// ------------------------------------------------------------------
// early-return paths
// ------------------------------------------------------------------

#[test]
fn parse_no_args_yields_defaults() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx"]), &mut buf),
        Intent::Run(default_config())
    );
}

#[test]
fn parse_help_flag_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-h"]), &mut buf), Intent::Help);
}

#[test]
fn parse_help_word_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "help"]), &mut buf), Intent::Help);
}

#[test]
fn parse_version_flag_returns_version() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-v"]), &mut buf), Intent::Version);
}

#[test]
fn parse_version_word_returns_version() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "version"]), &mut buf), Intent::Version);
}

#[test]
fn parse_unknown_argument_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "unknown"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("Unknown argument"));
}

// ------------------------------------------------------------------
// value flags
// ------------------------------------------------------------------

#[test]
fn parse_r_missing_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-r"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_n_missing_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-n"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_k_non_numeric_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "-k", "abc"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
}

#[test]
fn parse_k_zero_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-k", "0"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("between 1 and 255"));
}

#[test]
fn parse_j_flag_sets_jobs() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.jobs = Some(2);
    assert_eq!(
        parse(&args(&["rx", "-j", "2"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_c_flag_sets_cores() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.cores = Some(4);
    assert_eq!(
        parse(&args(&["rx", "-c", "4"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_jobs_long_sets_jobs() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.jobs = Some(3);
    assert_eq!(
        parse(&args(&["rx", "--jobs", "3"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_cores_long_sets_cores() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.cores = Some(5);
    assert_eq!(
        parse(&args(&["rx", "--cores", "5"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_jobs_zero_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-j", "0"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("between 1 and 65535"));
}

#[test]
fn parse_jobs_non_numeric_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--jobs", "abc"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
}

#[test]
fn parse_cores_zero_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--cores", "0"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("between 1 and 65535"));
}

#[test]
fn parse_r_with_value_sets_repo() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/path/to/repo");
    assert_eq!(
        parse(&args(&["rx", "-r", "/path/to/repo"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_n_with_value_sets_hostname() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.hostname = Some(String::from("myhost"));
    assert_eq!(
        parse(&args(&["rx", "-n", "myhost"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_k_with_value_sets_keep() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.keep = 5;
    assert_eq!(
        parse(&args(&["rx", "-k", "5"]), &mut buf),
        Intent::Run(expected)
    );
}

// ------------------------------------------------------------------
// boolean flags
// ------------------------------------------------------------------

#[test]
fn parse_d_flag_sets_diff() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.diff = true;
    assert_eq!(parse(&args(&["rx", "-d"]), &mut buf), Intent::Run(expected));
}

#[test]
fn parse_u_flag_sets_update() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.update = true;
    assert_eq!(parse(&args(&["rx", "-u"]), &mut buf), Intent::Run(expected));
}

#[test]
fn parse_y_flag_sets_yes() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.yes = true;
    assert_eq!(parse(&args(&["rx", "-y"]), &mut buf), Intent::Run(expected));
}

#[test]
fn parse_d_and_u_together() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.update = true;
    expected.diff = true;
    assert_eq!(
        parse(&args(&["rx", "-d", "-u"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_combined_flags_and_values() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/my/repo");
    expected.hostname = Some(String::from("myhost"));
    expected.keep = 3;
    expected.update = true;
    expected.diff = true;
    assert_eq!(
        parse(
            &args(&[
                "rx", "-d", "-u", "-r", "/my/repo", "-n", "myhost", "-k", "3",
            ]),
            &mut buf,
        ),
        Intent::Run(expected)
    );
}

#[test]
fn parse_k_then_h_early_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "-k", "5", "-h"]), &mut buf),
        Intent::Help
    );
}

#[test]
fn parse_unknown_flag_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-x"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
}

#[test]
fn parse_short_value_flag_must_be_last_in_cluster() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "-rk", "/r"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("must be last"));
}

#[test]
fn parse_short_boolean_then_value_in_cluster_ok() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.yes = true;
    expected.repo = String::from("/r");
    assert_eq!(
        parse(&args(&["rx", "-yr", "/r"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_short_value_rejects_flag_like_value() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "-r", "-d"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_long_value_rejects_flag_like_value() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--repo", "--diff"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

// ------------------------------------------------------------------
// long flags
// ------------------------------------------------------------------

#[test]
fn parse_help_long_returns_help() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--help"]), &mut buf), Intent::Help);
}

#[test]
fn parse_version_long_returns_version() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--version"]), &mut buf),
        Intent::Version
    );
}

#[test]
fn parse_update_long_sets_update() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.update = true;
    assert_eq!(
        parse(&args(&["rx", "--update"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_diff_long_sets_diff() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.diff = true;
    assert_eq!(
        parse(&args(&["rx", "--diff"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_yes_long_sets_yes() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.yes = true;
    assert_eq!(
        parse(&args(&["rx", "--yes"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_dry_run_long_sets_dry_run() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.dry_run = true;
    assert_eq!(
        parse(&args(&["rx", "--dry-run"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_no_color_flag_is_accepted() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--no-color"]), &mut buf),
        Intent::Run(default_config())
    );
}

#[test]
fn parse_repo_long_sets_repo() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/my/repo");
    assert_eq!(
        parse(&args(&["rx", "--repo", "/my/repo"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_hostname_long_sets_hostname() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.hostname = Some(String::from("myhost"));
    assert_eq!(
        parse(&args(&["rx", "--hostname", "myhost"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_keep_long_sets_keep() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.keep = 5;
    assert_eq!(
        parse(&args(&["rx", "--keep", "5"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_keep_long_non_numeric_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--keep", "abc"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
}

#[test]
fn parse_keep_long_zero_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--keep", "0"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("between 1 and 255"));
}

#[test]
fn parse_repo_long_missing_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--repo"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_unknown_long_flag_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--unknown"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
}

#[test]
fn parse_stray_argument_after_flags_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--yes", "bogus"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("Unknown argument"));
}

#[test]
fn parse_keep_overflow_reports_the_valid_range() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--keep", "300"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("between 1 and 255"));
}

#[test]
fn parse_jobs_overflow_reports_the_valid_range() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--jobs", "70000"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("between 1 and 65535"));
}

#[test]
fn parse_cores_overflow_reports_the_valid_range() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--cores", "70000"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("between 1 and 65535"));
}

#[test]
fn parse_repo_long_equals_sets_repo() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/x");
    assert_eq!(
        parse(&args(&["rx", "--repo=/x"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_value_keeps_embedded_equals_signs() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/x=a");
    assert_eq!(
        parse(&args(&["rx", "--repo=/x=a"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_value_long_equals_empty_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--repo="]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_boolean_long_with_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--yes=1"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("does not take a value"));
}

#[test]
fn parse_help_long_with_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--help=1"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("does not take a value"));
}

#[test]
fn parse_only_sets_the_step_selection() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.only = Some(vec![Step::Pull, Step::Diff]);
    assert_eq!(
        parse(&args(&["rx", "--only", "pull,diff"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_only_equals_accepts_a_single_step() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.only = Some(vec![Step::Rebuild]);
    assert_eq!(
        parse(&args(&["rx", "--only=rebuild"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_only_unknown_step_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--only", "pull,bogus"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("Unknown step"));
}

#[test]
fn parse_only_missing_value_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--only"]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_only_empty_equals_returns_error() {
    let mut buf = Vec::new();
    assert_eq!(parse(&args(&["rx", "--only="]), &mut buf), Intent::Invalid);
    assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
}

#[test]
fn parse_only_rejects_an_empty_item() {
    let mut buf = Vec::new();
    assert_eq!(
        parse(&args(&["rx", "--only", "pull,,diff"]), &mut buf),
        Intent::Invalid
    );
    assert!(String::from_utf8_lossy(&buf).contains("Unknown step"));
}

#[test]
fn parse_equals_value_does_not_swallow_the_next_flag() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.repo = String::from("/x");
    expected.yes = true;
    assert_eq!(
        parse(&args(&["rx", "--repo=/x", "-y"]), &mut buf),
        Intent::Run(expected)
    );
}

#[test]
fn parse_only_equals_does_not_swallow_the_next_flag() {
    let mut buf = Vec::new();
    let mut expected = default_config();
    expected.only = Some(vec![Step::Diff]);
    expected.yes = true;
    assert_eq!(
        parse(&args(&["rx", "--only=diff", "-y"]), &mut buf),
        Intent::Run(expected)
    );
}
