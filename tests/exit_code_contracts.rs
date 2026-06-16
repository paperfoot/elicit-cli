//! Verify the semantic exit-code contract (0-4).
//!
//! Uses the hidden `contract` command for deterministic triggers and real
//! commands for natural exit-code coverage. NONE of these tests hit the
//! network: API commands are exercised only on the no-key path (fail-fast
//! exit 2) or via clap parse errors (exit 3).

use assert_cmd::Command;

fn elicit() -> Command {
    Command::cargo_bin("elicit").unwrap()
}

// ── Contract command: deterministic 0-4 ────────────────────────────────────

#[test]
fn contract_exit_0() {
    elicit().args(["contract", "0"]).assert().code(0);
}

#[test]
fn contract_exit_1_transient() {
    elicit().args(["contract", "1"]).assert().code(1);
}

#[test]
fn contract_exit_2_config() {
    elicit().args(["contract", "2"]).assert().code(2);
}

#[test]
fn contract_exit_3_bad_input() {
    elicit().args(["contract", "3"]).assert().code(3);
}

#[test]
fn contract_exit_4_rate_limited() {
    elicit().args(["contract", "4"]).assert().code(4);
}

// ── Informational commands: exit 0 ─────────────────────────────────────────

#[test]
fn help_exits_0() {
    elicit().arg("--help").assert().code(0);
}

#[test]
fn version_exits_0() {
    elicit().arg("--version").assert().code(0);
}

#[test]
fn agent_info_exits_0() {
    elicit().arg("agent-info").assert().code(0);
}

#[test]
fn agent_info_alias_exits_0() {
    elicit().arg("info").assert().code(0);
}

#[test]
fn config_path_exits_0() {
    elicit().args(["config", "path"]).assert().code(0);
}

#[test]
fn config_show_exits_0() {
    elicit().args(["config", "show"]).assert().code(0);
}

// ── Parse errors: exit 3 ───────────────────────────────────────────────────

#[test]
fn missing_subcommand_exits_3() {
    // No subcommand at all is a parse error.
    elicit().assert().code(3);
}

#[test]
fn search_missing_query_exits_3() {
    // `search` requires a positional <query>.
    elicit().arg("search").assert().code(3);
}

#[test]
fn trials_missing_query_exits_3() {
    elicit().arg("trials").assert().code(3);
}

#[test]
fn report_missing_action_exits_3() {
    // `report` is a subcommand group; bare `report` is a parse error.
    elicit().arg("report").assert().code(3);
}

#[test]
fn invalid_corpus_value_exits_3() {
    // clap rejects an out-of-enum --corpus before any network call.
    elicit()
        .args(["search", "x", "--corpus", "nonsense"])
        .assert()
        .code(3);
}

// ── No-key config gate: exit 2 (fail-fast, no network) ─────────────────────

#[test]
fn search_without_key_exits_2() {
    // With no key resolvable, every API command must fail fast with exit 2
    // BEFORE any network call. We point base_url at an unroutable host as a
    // belt-and-suspenders guard, but the key gate fires first.
    elicit()
        .env_remove("ELICIT_API_KEY")
        .env("ELICIT_BASE_URL", "http://127.0.0.1:1/never")
        .env("HOME", tempfile::tempdir().unwrap().path())
        .args(["search", "anything"])
        .assert()
        .code(2);
}
