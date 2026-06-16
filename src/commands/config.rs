use serde::Serialize;

use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

// ── config show ────────────────────────────────────────────────────────────
//
// The API key is NEVER shown in plain text. We serialize a redacted view that
// reports the masked value and where it was resolved from (flag/env/config),
// while still showing every non-secret setting verbatim.

#[derive(Serialize)]
struct RedactedConfig<'a> {
    base_url: &'a str,
    keys: RedactedKeys,
    update: &'a crate::config::UpdateConfig,
}

#[derive(Serialize)]
struct RedactedKeys {
    /// Masked key (or "(not set)") -- never the raw value.
    api_key: String,
    /// Where the key was resolved from: flag, env, config, or none.
    api_key_source: &'static str,
}

pub fn show(ctx: Ctx, api_key: Option<&str>, config: &AppConfig) -> Result<(), AppError> {
    let (resolved, source) = config::resolve_api_key_opt(api_key, config);
    let masked = resolved
        .as_deref()
        .map(config::mask_secret)
        .unwrap_or_else(|| "(not set)".to_string());

    let view = RedactedConfig {
        base_url: &config.base_url,
        keys: RedactedKeys {
            api_key: masked,
            api_key_source: source,
        },
        update: &config.update,
    };

    output::print_success_or(ctx, &view, |c| {
        println!("{}", serde_json::to_string_pretty(c).unwrap());
    });
    Ok(())
}

// ── config path ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ConfigPath {
    path: String,
    exists: bool,
}

pub fn path(ctx: Ctx) -> Result<(), AppError> {
    let p = config::config_path();
    let result = ConfigPath {
        path: p.display().to_string(),
        exists: p.exists(),
    };
    output::print_success_or(ctx, &result, |r| {
        println!("{}", r.path);
        if !r.exists {
            use owo_colors::OwoColorize;
            println!("  {}", "(file does not exist, using defaults)".dimmed());
        }
    });
    Ok(())
}
