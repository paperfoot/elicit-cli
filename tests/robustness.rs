//! Robustness tests: verify recovery from bad state.
//!
//! These tests ensure discovery and diagnostic commands work even when
//! configuration is malformed, and that enforced constraints match agent-info.
//! None of these tests hit the network.

use assert_cmd::Command;

fn elicit() -> Command {
    Command::cargo_bin("elicit").unwrap()
}

/// Where the config file lives under a temp HOME on macOS (the platform these
/// tests run on). `directories` maps to ~/Library/Application Support/elicit.
fn macos_config_dir(home: &std::path::Path) -> std::path::PathBuf {
    home.join("Library/Application Support/elicit")
}

// ── Malformed config resilience ────────────────────────────────────────────

/// agent-info must work even with a broken config file (it never loads config).
#[test]
fn agent_info_works_with_malformed_config() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = macos_config_dir(tmp.path());
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "{{invalid toml").unwrap();

    elicit()
        .env("HOME", tmp.path())
        .arg("agent-info")
        .assert()
        .code(0);
}

/// config path must work even with a broken config file.
#[test]
fn config_path_works_with_malformed_config() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = macos_config_dir(tmp.path());
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "{{invalid toml").unwrap();

    elicit()
        .env("HOME", tmp.path())
        .args(["config", "path"])
        .assert()
        .code(0);
}

/// config show should fail gracefully with exit 2 on malformed config.
#[test]
fn config_show_fails_with_malformed_config() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = macos_config_dir(tmp.path());
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "{{invalid toml").unwrap();

    elicit()
        .env("HOME", tmp.path())
        .args(["config", "show"])
        .assert()
        .code(2);
}

// ── Constraint enforcement ─────────────────────────────────────────────────

/// Invalid --retracted value should be rejected by clap (exit 3) before any
/// network call.
#[test]
fn invalid_retracted_rejected() {
    elicit()
        .args(["search", "x", "--retracted", "nonsense"])
        .assert()
        .code(3);
}

/// Invalid --mode value should be rejected by clap (exit 3).
#[test]
fn invalid_mode_rejected() {
    elicit()
        .args(["trials", "x", "--mode", "fuzzy"])
        .assert()
        .code(3);
}

/// doctor must run (and exit 2 with no key) even under an unusual HOME, and
/// must never hang. Offline-safe: no key means no network probe.
#[test]
fn doctor_offline_safe_with_temp_home() {
    let tmp = tempfile::tempdir().unwrap();
    elicit()
        .env_remove("ELICIT_API_KEY")
        .env("HOME", tmp.path())
        .arg("doctor")
        .assert()
        .code(2);
}
