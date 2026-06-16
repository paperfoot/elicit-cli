//! `doctor` -- validate that the CLI can actually talk to Elicit right now.
//!
//! OFFLINE-SAFE: with no API key, the api_key check fails and the command exits
//! 2 WITHOUT touching the network. Only when a key is present do we attempt a
//! single, timeout-bounded reachability probe. The command never hangs and
//! never panics.

use serde::Serialize;

use crate::api::ElicitClient;
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct DoctorCheck {
    name: &'static str,
    status: CheckStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
}

#[derive(Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Serialize)]
struct DoctorSummary {
    pass: usize,
    warn: usize,
    fail: usize,
}

#[derive(Serialize)]
struct DoctorReport {
    checks: Vec<DoctorCheck>,
    summary: DoctorSummary,
}

pub fn run(ctx: Ctx, api_key: Option<&str>, config: &AppConfig) -> Result<(), AppError> {
    let mut checks: Vec<DoctorCheck> = Vec::new();

    // 1. config file presence (informational).
    let cfg_path = config::config_path();
    checks.push(if cfg_path.exists() {
        DoctorCheck {
            name: "config_file",
            status: CheckStatus::Pass,
            message: cfg_path.display().to_string(),
            suggestion: None,
        }
    } else {
        DoctorCheck {
            name: "config_file",
            status: CheckStatus::Warn,
            message: format!("{} not found (using defaults)", cfg_path.display()),
            suggestion: Some("Defaults are fine; create the file only to persist settings.".into()),
        }
    });

    // 2. base_url (informational).
    checks.push(DoctorCheck {
        name: "base_url",
        status: CheckStatus::Pass,
        message: config.base_url.clone(),
        suggestion: None,
    });

    // 3. api_key presence + elk_ format. This is the gate.
    let (key_opt, source) = config::resolve_api_key_opt(api_key, config);
    let key_present = match &key_opt {
        Some(key) => {
            let looks_valid = key.starts_with("elk_");
            checks.push(DoctorCheck {
                name: "api_key",
                status: if looks_valid {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Warn
                },
                message: format!("found via {source} ({})", config::mask_secret(key)),
                suggestion: if looks_valid {
                    None
                } else {
                    Some(
                        "Elicit keys normally start with elk_live_ — double-check the value."
                            .into(),
                    )
                },
            });
            true
        }
        None => {
            checks.push(DoctorCheck {
                name: "api_key",
                status: CheckStatus::Fail,
                message: "no API key (checked --api-key, ELICIT_API_KEY, config keys.api_key)"
                    .into(),
                suggestion: Some(
                    "Set ELICIT_API_KEY=elk_live_... from https://elicit.com/settings".into(),
                ),
            });
            false
        }
    };

    // 4. reachability + plan/quota -- ONLY when a key is present (offline-safe).
    if key_present {
        let key = key_opt.as_deref().unwrap_or_default();
        match ElicitClient::new(&config.base_url, key) {
            Ok(client) => match client.probe() {
                Ok(probe) if probe.reachable => {
                    let status_code = probe.status.unwrap_or(0);
                    let authed = status_code == 200;
                    checks.push(DoctorCheck {
                        name: "api_reachable",
                        status: CheckStatus::Pass,
                        message: format!("reachable (HTTP {status_code})"),
                        suggestion: None,
                    });
                    // Auth check derived from the probe status.
                    checks.push(match status_code {
                        200 => DoctorCheck {
                            name: "api_auth",
                            status: CheckStatus::Pass,
                            message: "key accepted".into(),
                            suggestion: None,
                        },
                        401 | 403 => DoctorCheck {
                            name: "api_auth",
                            status: CheckStatus::Fail,
                            message: format!("key rejected (HTTP {status_code})"),
                            suggestion: Some(
                                "Verify ELICIT_API_KEY at https://elicit.com/settings; reports/search need Pro+.".into(),
                            ),
                        },
                        other => DoctorCheck {
                            name: "api_auth",
                            status: CheckStatus::Warn,
                            message: format!("unexpected status HTTP {other}"),
                            suggestion: None,
                        },
                    });
                    // Plan / quota from rate-limit headers (absent => Enterprise/unlimited).
                    if authed {
                        let rl = &probe.rate_limit;
                        let msg = if rl.is_empty() {
                            "no rate-limit headers (unlimited / Enterprise tier)".to_string()
                        } else {
                            format!(
                                "{}/{} requests remaining (resets at epoch {})",
                                rl.remaining
                                    .map(|r| r.to_string())
                                    .unwrap_or_else(|| "?".into()),
                                rl.limit
                                    .map(|l| l.to_string())
                                    .unwrap_or_else(|| "?".into()),
                                rl.reset
                                    .map(|r| r.to_string())
                                    .unwrap_or_else(|| "?".into()),
                            )
                        };
                        checks.push(DoctorCheck {
                            name: "plan_quota",
                            status: CheckStatus::Pass,
                            message: msg,
                            suggestion: None,
                        });
                    }
                }
                Ok(_) => {
                    checks.push(DoctorCheck {
                        name: "api_reachable",
                        status: CheckStatus::Fail,
                        message: format!("could not reach {}", config.base_url),
                        suggestion: Some(
                            "Check your network. Override the endpoint with ELICIT_BASE_URL if needed.".into(),
                        ),
                    });
                }
                Err(e) => {
                    checks.push(DoctorCheck {
                        name: "api_reachable",
                        status: CheckStatus::Fail,
                        message: e.to_string(),
                        suggestion: Some("Check your network connection and retry.".into()),
                    });
                }
            },
            Err(e) => {
                checks.push(DoctorCheck {
                    name: "api_reachable",
                    status: CheckStatus::Fail,
                    message: e.to_string(),
                    suggestion: Some("Retry the command.".into()),
                });
            }
        }
    }

    let summary = DoctorSummary {
        pass: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Pass)
            .count(),
        warn: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count(),
        fail: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count(),
    };
    let has_failures = summary.fail > 0;
    let report = DoctorReport { checks, summary };

    output::print_success_or(ctx, &report, |r| {
        use owo_colors::OwoColorize;
        for check in &r.checks {
            let icon = match check.status {
                CheckStatus::Pass => "✓".green().to_string(),
                CheckStatus::Warn => "!".yellow().to_string(),
                CheckStatus::Fail => "✗".red().to_string(),
            };
            eprintln!("  {icon} {}: {}", check.name, check.message);
            if let Some(s) = &check.suggestion {
                eprintln!("      {}", s.dimmed());
            }
        }
        eprintln!(
            "  {} pass, {} warn, {} fail",
            r.summary.pass, r.summary.warn, r.summary.fail
        );
    });

    if has_failures {
        return Err(AppError::config_with(
            "doctor found issues",
            "Resolve the failing checks above (run with --json for machine-readable detail).",
        ));
    }
    Ok(())
}
