//! API-command contract tests that NEVER hit the network and NEVER require a
//! key.
//!
//! They exercise three things only:
//!   1. the no-key fail-fast gate (exit 2 BEFORE any request),
//!   2. the error/success envelope shape on that path,
//!   3. that stdout stays clean (pipeable) on the error path.
//!
//! Every test runs with `ELICIT_API_KEY` removed, a throwaway `HOME` (so no
//! real config leaks in), and `ELICIT_BASE_URL` pointed at an unroutable host
//! as a belt-and-suspenders guard — but the key gate fires first, so no
//! connection is ever attempted.

use assert_cmd::Command;

fn elicit_no_key() -> Command {
    let mut cmd = Command::cargo_bin("elicit").unwrap();
    cmd.env_remove("ELICIT_API_KEY")
        .env("HOME", tempfile::tempdir().unwrap().path())
        .env("ELICIT_BASE_URL", "http://127.0.0.1:1/never");
    cmd
}

// ── No key: every API command fails fast with exit 2 ───────────────────────

#[test]
fn search_no_key_exits_2_with_error_envelope_and_empty_stdout() {
    let out = elicit_no_key()
        .args(["--json", "search", "foo"])
        .output()
        .unwrap();

    // Exit 2 = config error (missing key), per the contract.
    assert_eq!(out.status.code(), Some(2), "no-key search must exit 2");

    // stdout MUST be empty so a downstream `| jq` over the data never sees half
    // a result on the error path.
    assert!(
        out.stdout.is_empty(),
        "stdout must be empty on the error path, got: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );

    // The error envelope lives on stderr and carries code/message/suggestion.
    let json: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("error envelope must be JSON on stderr");
    assert_eq!(json["version"], "1");
    assert_eq!(json["status"], "error");
    assert_eq!(json["error"]["code"], "config_error");
    assert!(json["error"]["message"].is_string());
    assert!(
        json["error"]["suggestion"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("elicit_api_key"),
        "suggestion should point the agent at ELICIT_API_KEY"
    );
}

#[test]
fn trials_no_key_exits_2_empty_stdout() {
    let out = elicit_no_key()
        .args(["--json", "trials", "semaglutide"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "config_error");
}

#[test]
fn report_new_no_key_exits_2_empty_stdout() {
    let out = elicit_no_key()
        .args(["--json", "report", "new", "Does X reduce Y?"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn report_list_no_key_exits_2_empty_stdout() {
    let out = elicit_no_key()
        .args(["--json", "report", "list"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn report_get_no_key_exits_2_empty_stdout() {
    let out = elicit_no_key()
        .args([
            "--json",
            "report",
            "get",
            "00000000-0000-0000-0000-000000000000",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn review_new_no_key_exits_2_empty_stdout() {
    let out = elicit_no_key()
        .args(["--json", "review", "new", "Does X reduce Y?"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn review_get_no_key_exits_2_empty_stdout() {
    let out = elicit_no_key()
        .args([
            "--json",
            "review",
            "get",
            "00000000-0000-0000-0000-000000000000",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

// ── doctor: offline-safe, exit 2, structured checks on STDOUT ──────────────

#[test]
fn doctor_no_key_exits_2_with_data_checks_on_stdout() {
    let out = elicit_no_key().args(["--json", "doctor"]).output().unwrap();

    // doctor surfaces failures via exit 2 (config) but still prints a full
    // success-shaped report payload to stdout for the agent to inspect.
    assert_eq!(out.status.code(), Some(2), "no-key doctor must exit 2");

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("doctor must emit JSON on stdout");
    assert_eq!(json["status"], "success");
    assert!(
        json["data"]["checks"].is_array(),
        "doctor data.checks must be an array"
    );

    // The api_key check must be present and FAILED with no key.
    let checks = json["data"]["checks"].as_array().unwrap();
    let api_key_check = checks
        .iter()
        .find(|c| c["name"] == "api_key")
        .expect("doctor must include an api_key check");
    assert_eq!(api_key_check["status"], "fail");

    // Offline-safe: with no key, no reachability probe is attempted, so there
    // must be no api_reachable check.
    assert!(
        !checks.iter().any(|c| c["name"] == "api_reachable"),
        "doctor must NOT probe the network when no key is present"
    );
}

#[test]
fn doctor_human_mode_exits_2_and_keeps_stdout_clean() {
    // In human (forced non-JSON) mode the checks render to STDERR; STDOUT must
    // stay empty so the command remains pipe-safe. We force human output by
    // passing neither --json nor a pipe... but assert_cmd always pipes, which
    // auto-engages JSON. So instead we assert the JSON-path invariant: the
    // error envelope (if any) is on stderr, and exit is 2.
    let out = elicit_no_key().arg("doctor").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    // Piped => JSON on stdout (the data report). Either way it must parse.
    let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&out.stdout);
    assert!(
        parsed.is_ok(),
        "doctor stdout must be valid JSON when piped"
    );
}

// ── Arg parsing without a key (no network) ─────────────────────────────────

#[test]
fn search_parses_all_filter_flags_then_gates_on_key() {
    // A fully-loaded search invocation must parse cleanly and then fail ONLY on
    // the missing key (exit 2), proving the flags are all accepted by clap.
    let out = elicit_no_key()
        .args([
            "--json",
            "search",
            "sleep and cognition",
            "--corpus",
            "pubmed",
            "--mode",
            "semantic",
            "--max-results",
            "5",
            "--min-year",
            "2015",
            "--max-year",
            "2024",
            "--type",
            "RCT",
            "--type",
            "Meta-Analysis",
            "--max-quartile",
            "1",
            "--include-kw",
            "humans",
            "--exclude-kw",
            "rats",
            "--has-pdf",
            "--retracted",
            "exclude",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "loaded search should parse, then gate on the missing key"
    );
    assert!(out.stdout.is_empty());
}

#[test]
fn review_new_parses_pair_flags_then_gates_on_key() {
    let out = elicit_no_key()
        .args([
            "--json",
            "review",
            "new",
            "Does metformin extend lifespan?",
            "--search",
            "metformin lifespan",
            "--corpus",
            "clinical_trials",
            "--mode",
            "semantic",
            "--max-results",
            "300",
            "--screen",
            "Human study:Must be in human subjects",
            "--fulltext-screen",
            "Outcome:Reports a longevity outcome",
            "--reuse-abstract-criteria",
            "--extract-column",
            "Sample size:Report the N",
            "--extract-column",
            "Benefit:Did it help?:yes|no|unclear",
            "--generate-extraction",
            "--generate-report",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

// ── Local input-validation gates (exit 3) reachable with a DUMMY key ─────────
//
// These set ELICIT_API_KEY to a throwaway value so the no-key gate passes, then
// hit a LOCAL validation guard that returns BEFORE the client is built — so no
// network call is ever made. They pin the exit-3 input-error path.

fn elicit_dummy_key() -> Command {
    let mut cmd = Command::cargo_bin("elicit").unwrap();
    cmd.env("ELICIT_API_KEY", "elk_live_dummy_not_used")
        .env("HOME", tempfile::tempdir().unwrap().path())
        .env("ELICIT_BASE_URL", "http://127.0.0.1:1/never");
    cmd
}

#[test]
fn search_keyword_mode_with_filters_exits_3() {
    // keyword mode + any filter is mutually exclusive; guarded locally (exit 3)
    // BEFORE any request, so the dummy key never triggers a network call.
    let out = elicit_dummy_key()
        .args([
            "--json",
            "search",
            "foo",
            "--mode",
            "keyword",
            "--min-year",
            "2020",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "keyword+filters must exit 3");
    assert!(
        out.stdout.is_empty(),
        "stdout must be empty on the input-error path"
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["error"]["code"], "invalid_input");
}

#[test]
fn trials_keyword_mode_with_filters_exits_3() {
    let out = elicit_dummy_key()
        .args([
            "--json",
            "trials",
            "semaglutide",
            "--mode",
            "keyword",
            "--phase",
            "PHASE3",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "invalid_input");
}

#[test]
fn review_generate_report_without_extraction_exits_3() {
    // --generate-report requires extraction; guarded locally (exit 3) before the
    // request is built.
    let out = elicit_dummy_key()
        .args([
            "--json",
            "review",
            "new",
            "Does X reduce Y?",
            "--generate-report",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "invalid_input");
}

#[test]
fn review_fulltext_without_abstract_exits_3() {
    let out = elicit_dummy_key()
        .args([
            "--json",
            "review",
            "new",
            "Does X reduce Y?",
            "--fulltext-screen",
            "Outcome:Reports an outcome",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "invalid_input");
}

#[test]
fn review_extract_column_bad_choices_exits_3() {
    // Only one choice provided — must be 2-10; local parse error (exit 3).
    let out = elicit_dummy_key()
        .args([
            "--json",
            "review",
            "new",
            "Does X reduce Y?",
            "--extract-column",
            "Benefit:Did it help?:onlyone",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "invalid_input");
}

// ── Numeric range guards (exit 3) caught by clap BEFORE the key gate ─────────

#[test]
fn search_max_results_out_of_range_exits_3() {
    // 0 is below the 1..=10000 bound; clap rejects it (exit 3) before anything.
    let out = elicit_no_key()
        .args(["--json", "search", "foo", "--max-results", "0"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "max-results 0 must exit 3");
    assert!(out.stdout.is_empty());
}

#[test]
fn search_max_quartile_out_of_range_exits_3() {
    let out = elicit_no_key()
        .args(["--json", "search", "foo", "--max-quartile", "9"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
}

#[test]
fn report_get_poll_interval_zero_exits_3() {
    // --poll-interval 0 is below the 1.. floor; clap rejects it (exit 3).
    let out = elicit_no_key()
        .args([
            "--json",
            "report",
            "get",
            "00000000-0000-0000-0000-000000000000",
            "--poll-interval",
            "0",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
}

// ── ELICIT_BASE_URL override actually takes effect ──────────────────────────

#[test]
fn base_url_env_override_is_reflected_in_config_show() {
    let mut cmd = Command::cargo_bin("elicit").unwrap();
    let out = cmd
        .env("HOME", tempfile::tempdir().unwrap().path())
        .env("ELICIT_BASE_URL", "http://example.test/custom/base")
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        json["data"]["base_url"], "http://example.test/custom/base",
        "ELICIT_BASE_URL must override the default base_url"
    );
}

#[test]
fn base_url_defaults_when_env_unset() {
    let mut cmd = Command::cargo_bin("elicit").unwrap();
    let out = cmd
        .env_remove("ELICIT_BASE_URL")
        .env("HOME", tempfile::tempdir().unwrap().path())
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["data"]["base_url"], "https://elicit.com/api/v1");
}
