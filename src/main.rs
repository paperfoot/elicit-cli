//! elicit -- agent-grade CLI for the Elicit research API.
//!
//! Built on the agent-cli-framework patterns:
//!   - Modular structure (api, cli, config, error, output, commands/)
//!   - JSON envelope on stdout, coloured human output on a TTY
//!   - Semantic exit codes (0-4); HTTP status mapped to the right code
//!   - `--json` / `--quiet` / `--api-key` global flags
//!   - `agent-info` for machine-readable capability discovery
//!   - `doctor` for offline-safe key + reachability diagnostics
//!   - `config show/path`, `skill install`, distribution-aware `update`

mod api;
mod cli;
mod commands;
mod config;
mod error;
mod output;

use clap::Parser;

use cli::{Cli, Commands, ConfigAction, SkillAction};
use output::{Ctx, Format};

/// Pre-scan argv for --json before clap parses. Ensures --json is honored on
/// help, version, and parse-error paths where the Cli struct isn't populated.
fn has_json_flag() -> bool {
    std::env::args_os().any(|a| a == "--json")
}

fn main() {
    let json_flag = has_json_flag();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Help and --version are NOT errors. Exit 0.
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let format = Format::detect(json_flag);
                match format {
                    Format::Json => {
                        output::print_help_json(e);
                        std::process::exit(0);
                    }
                    Format::Human => e.exit(),
                }
            }
            // Actual parse errors -- always exit 3, never let clap own the exit.
            let format = Format::detect(json_flag);
            output::print_clap_error(format, &e);
            std::process::exit(3);
        }
    };

    let ctx = Ctx::new(cli.json, cli.quiet);
    let api_key = cli.api_key.as_deref();

    // Config is loaded lazily per command. agent-info, config path, and skill
    // never need it and must work even with a malformed config file.
    let result = match cli.command {
        // ── API commands ───────────────────────────────────────────────────
        Commands::Search(args) => {
            config::load().and_then(|cfg| commands::search::run(ctx, args, api_key, &cfg))
        }
        Commands::Trials(args) => {
            config::load().and_then(|cfg| commands::trials::run(ctx, args, api_key, &cfg))
        }
        Commands::Report { action } => {
            config::load().and_then(|cfg| commands::report::run(ctx, action, api_key, &cfg))
        }
        Commands::Review { action } => {
            config::load().and_then(|cfg| commands::review::run(ctx, action, api_key, &cfg))
        }
        Commands::Doctor => {
            config::load().and_then(|cfg| commands::doctor::run(ctx, api_key, &cfg))
        }

        // ── Built-ins ──────────────────────────────────────────────────────
        Commands::AgentInfo => {
            commands::agent_info::run();
            Ok(())
        }
        Commands::Skill { action } => match action {
            SkillAction::Install => commands::skill::install(ctx),
            SkillAction::Status => commands::skill::status(ctx),
        },
        Commands::Config { action } => match action {
            ConfigAction::Show => {
                config::load().and_then(|cfg| commands::config::show(ctx, api_key, &cfg))
            }
            ConfigAction::Path => commands::config::path(ctx),
        },
        Commands::Update { check } => {
            config::load().and_then(|cfg| commands::update::run(ctx, check, &cfg))
        }
        Commands::Contract { code } => commands::contract::run(ctx, code),
    };

    if let Err(e) = result {
        output::print_error(ctx.format, &e);
        std::process::exit(e.exit_code());
    }
}
