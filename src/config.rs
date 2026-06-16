/// Configuration loading with 3-tier precedence:
///   1. Compiled defaults
///   2. TOML config file (~/.config/elicit/config.toml)
///   3. Environment variables (ELICIT_*)
///
/// The API key is the one exception to figment env-splitting: it is resolved
/// separately via `resolve_api_key` (flag -> ELICIT_API_KEY -> config) so the
/// secret never has to round-trip through a split env provider where an
/// underscore in the key would corrupt the parse.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::AppError;

// ── Config structs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Base URL for the Elicit API (no trailing slash).
    pub base_url: String,

    /// Credentials.
    pub keys: Keys,

    /// Update settings.
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Keys {
    /// Elicit API key (elk_live_...). Resolved via `resolve_api_key`; prefer
    /// the ELICIT_API_KEY environment variable over storing it here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Enable or disable update checks/apply.
    pub enabled: bool,

    /// Install source: auto, standalone, homebrew, cargo, cargo_binstall,
    /// npm, bun, uv_tool, pipx, winget, scoop, apt, managed, or unknown.
    #[serde(alias = "source")]
    pub install_source: String,

    /// GitHub repository owner.
    pub owner: String,

    /// GitHub repository name.
    pub repo: String,

    /// crates.io package name.
    pub crate_name: String,

    /// Homebrew formula name.
    pub formula: String,

    /// Optional Homebrew tap, for example owner/tap.
    pub tap: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            keys: Keys::default(),
            update: UpdateConfig::default(),
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            install_source: "auto".into(),
            owner: "paperfoot".into(),
            repo: "elicit-cli".into(),
            crate_name: "elicit".into(),
            formula: "elicit".into(),
            tap: "paperfoot/tap".into(),
        }
    }
}

pub fn default_base_url() -> String {
    "https://elicit.com/api/v1".into()
}

// ── Paths ──────────────────────────────────────────────────────────────────

pub fn config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.toml")
}

// ── Loading ────────────────────────────────────────────────────────────────

pub fn load() -> Result<AppConfig, AppError> {
    use figment::Figment;
    use figment::providers::{Env, Format as _, Serialized, Toml};

    // NOTE: two ELICIT_* vars are intentionally NOT routed through the split env
    // provider:
    //   - ELICIT_API_KEY: `.split("_")` would turn it into key path `api.key`
    //     and mangle underscores inside the secret. Resolved separately by
    //     `resolve_api_key`.
    //   - ELICIT_BASE_URL: `.split("_")` would map it to the nested path
    //     `base.url`, which does not match the flat struct field `base_url`, so
    //     the override would be silently dropped. We resolve it explicitly
    //     below (mirroring the API key) and layer it as a single flat value.
    // Everything else (e.g. ELICIT_UPDATE_*) still flows through the split
    // provider unchanged.
    let mut figment = Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(config_path()))
        .merge(
            Env::prefixed("ELICIT_")
                .split("_")
                .ignore(&["ELICIT_API_KEY", "ELICIT_BASE_URL"]),
        );

    // Explicit flat override for ELICIT_BASE_URL so the documented endpoint
    // retargeting (staging/proxy) actually takes effect.
    if let Ok(v) = std::env::var("ELICIT_BASE_URL") {
        let v = v.trim();
        if !v.is_empty() {
            figment = figment.merge(Serialized::default("base_url", v));
        }
    }

    figment
        .extract()
        .map_err(|e| AppError::Config(e.to_string()))
}

// ── API key resolution ──────────────────────────────────────────────────────

/// Resolve the Elicit API key from, in order:
///   1. the `--api-key` flag value
///   2. the `ELICIT_API_KEY` environment variable
///   3. the `keys.api_key` config value
///
/// Returns `AppError::Config` (exit 2) with an actionable suggestion when no
/// key is found. This is the fail-fast gate every networked command calls
/// BEFORE making any request.
pub fn resolve_api_key(flag: Option<&str>, config: &AppConfig) -> Result<String, AppError> {
    if let Some(v) = flag {
        let v = v.trim();
        if !v.is_empty() {
            return Ok(v.to_string());
        }
    }
    if let Ok(v) = std::env::var("ELICIT_API_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Ok(v);
        }
    }
    if let Some(v) = config.keys.api_key.as_deref() {
        let v = v.trim();
        if !v.is_empty() {
            return Ok(v.to_string());
        }
    }
    Err(AppError::Config(
        "no Elicit API key found (checked --api-key, ELICIT_API_KEY, and config keys.api_key)"
            .into(),
    ))
}

/// Resolve the API key for display/diagnostics WITHOUT erroring when absent.
/// Returns `(value, source)` where source is "flag", "env", "config", or
/// "none".
pub fn resolve_api_key_opt(
    flag: Option<&str>,
    config: &AppConfig,
) -> (Option<String>, &'static str) {
    if let Some(v) = flag {
        let v = v.trim();
        if !v.is_empty() {
            return (Some(v.to_string()), "flag");
        }
    }
    if let Ok(v) = std::env::var("ELICIT_API_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return (Some(v), "env");
        }
    }
    if let Some(v) = config.keys.api_key.as_deref() {
        let v = v.trim();
        if !v.is_empty() {
            return (Some(v.to_string()), "config");
        }
    }
    (None, "none")
}

// ── Secret masking ──────────────────────────────────────────────────────────

/// Mask a secret for display: "elk_...1234". Uses char boundaries (not byte
/// offsets) to avoid panics on non-ASCII input.
pub fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        let prefix: String = chars[..2.min(chars.len())].iter().collect();
        format!("{prefix}***")
    } else {
        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{prefix}...{suffix}")
    }
}
