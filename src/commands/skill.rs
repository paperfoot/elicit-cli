use serde::Serialize;
use std::path::PathBuf;

use crate::error::AppError;
use crate::output::{self, Ctx};

// ── Skill content ───────────────────────────────────────────────────────────
// Built from the binary name. No hardcoded app name.

fn skill_content() -> String {
    let name = env!("CARGO_BIN_NAME");
    format!(
        r#"---
name: {name}
description: >
  Search academic papers and clinical trials, and run AI research reports and
  systematic reviews via the Elicit API. Use when the user wants to search
  papers, run a literature search, find studies or clinical trials, generate a
  research report, run a systematic review, or asks "what does the research
  say" about a topic. Run `{name} agent-info` for the full capability manifest.
---

## {name}

Agent-grade CLI for the Elicit research API (138M+ papers, clinical trials,
async reports and systematic reviews).

Triggers: search papers, literature search, find studies, find clinical
trials, research report, systematic review, "what does the research say".

First, discover everything:

```
{name} agent-info
```

Setup: export `ELICIT_API_KEY=elk_live_...` (from https://elicit.com/settings),
then `{name} doctor` to verify. Output is JSON when piped; add `--json` to
force it. Exit codes: 0 ok, 1 retry, 2 config/auth, 3 bad input, 4 rate limited.

Common calls:

```
{name} search "effects of sleep deprivation on cognition" --max-results 5
{name} trials "semaglutide obesity" --phase PHASE3 --status RECRUITING
{name} report new "Do GLP-1 agonists reduce MACE?" --wait
{name} review get <reviewId> --download csv
```
"#
    )
}

// ── Platform targets ────────────────────────────────────────────────────────

struct SkillTarget {
    name: &'static str,
    path: PathBuf,
}

fn home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn skill_targets() -> Vec<SkillTarget> {
    let h = home();
    let app = env!("CARGO_BIN_NAME");
    vec![
        SkillTarget {
            name: "Claude Code",
            path: h.join(format!(".claude/skills/{app}")),
        },
        SkillTarget {
            name: "Codex CLI",
            path: h.join(format!(".codex/skills/{app}")),
        },
        SkillTarget {
            name: "Gemini CLI",
            path: h.join(format!(".gemini/skills/{app}")),
        },
    ]
}

// ── Install ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct InstallResult {
    platform: String,
    path: String,
    status: String,
}

pub fn install(ctx: Ctx) -> Result<(), AppError> {
    let content = skill_content();
    let mut results: Vec<InstallResult> = Vec::new();

    for target in &skill_targets() {
        let skill_path = target.path.join("SKILL.md");

        if skill_path.exists() && std::fs::read_to_string(&skill_path).is_ok_and(|c| c == content) {
            results.push(InstallResult {
                platform: target.name.into(),
                path: skill_path.display().to_string(),
                status: "already_current".into(),
            });
            continue;
        }

        std::fs::create_dir_all(&target.path)?;
        std::fs::write(&skill_path, &content)?;
        results.push(InstallResult {
            platform: target.name.into(),
            path: skill_path.display().to_string(),
            status: "installed".into(),
        });
    }

    output::print_success_or(ctx, &results, |r| {
        use owo_colors::OwoColorize;
        for item in r {
            let marker = if item.status == "installed" { "+" } else { "=" };
            println!(
                " {} {} -> {}",
                marker.green(),
                item.platform.bold(),
                item.path.dimmed()
            );
        }
    });

    Ok(())
}

// ── Status ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SkillStatus {
    platform: String,
    installed: bool,
    current: bool,
}

pub fn status(ctx: Ctx) -> Result<(), AppError> {
    let content = skill_content();
    let mut results: Vec<SkillStatus> = Vec::new();

    for target in &skill_targets() {
        let skill_path = target.path.join("SKILL.md");
        let (installed, current) = if skill_path.exists() {
            let current = std::fs::read_to_string(&skill_path).is_ok_and(|c| c == content);
            (true, current)
        } else {
            (false, false)
        };
        results.push(SkillStatus {
            platform: target.name.into(),
            installed,
            current,
        });
    }

    output::print_success_or(ctx, &results, |r| {
        use owo_colors::OwoColorize;
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Platform", "Installed", "Current"]);
        for item in r {
            table.add_row(vec![
                item.platform.clone(),
                if item.installed {
                    "Yes".green().to_string()
                } else {
                    "No".red().to_string()
                },
                if item.current {
                    "Yes".green().to_string()
                } else {
                    "No".dimmed().to_string()
                },
            ]);
        }
        println!("{table}");
    });

    Ok(())
}
