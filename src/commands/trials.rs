use crate::api::ElicitClient;
use crate::api::models::*;
use crate::cli::TrialsArgs;
use crate::commands::search::map_mode;
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

pub fn run(ctx: Ctx, args: TrialsArgs, api_key: Option<&str>, config: &AppConfig) -> Result<(), AppError> {
    let key = config::resolve_api_key(api_key, config)?;

    if args.query.trim().is_empty() {
        return Err(AppError::invalid_with(
            "query cannot be empty",
            "Provide a non-empty search query as the first argument",
        ));
    }

    let mode = args.mode.map(map_mode);

    let mut filters = TrialFilters {
        phase: args.phase.clone(),
        recruitment_status: args.status.clone(),
        ..Default::default()
    };
    if args.has_results {
        filters.has_results = Some(true);
    }

    if mode == Some(SearchMode::Keyword) && !filters.is_empty() {
        return Err(AppError::invalid_with(
            "keyword search mode cannot be combined with trial filters",
            "Use --mode semantic with filters, or move filter expressions into the query string and drop the filter flags",
        ));
    }

    let req = TrialSearchRequest {
        query: args.query.clone(),
        search_mode: mode,
        max_results: args.max_results,
        trial_filters: if filters.is_empty() { None } else { Some(filters) },
    };

    let client = ElicitClient::new(&config.base_url, &key)?;
    let outcome = client.search_trials(&req)?;

    output::emit_rate_limit(ctx, &outcome.rate_limit);

    let body = outcome.body;
    output::print_success_or(ctx, &body, render_human);

    Ok(())
}

fn render_human(resp: &TrialSearchResponse) {
    use owo_colors::OwoColorize;

    if resp.trials.is_empty() {
        println!("{}", "No trials found.".dimmed());
    } else {
        for (i, t) in resp.trials.iter().enumerate() {
            println!(
                "{} {} {}",
                format!("{}.", i + 1).dimmed(),
                t.nct_id.cyan(),
                t.title.bold()
            );
            let status = t.overall_status.as_deref().unwrap_or("unknown status");
            let phase = if t.phase.is_empty() {
                "N/A".to_string()
            } else {
                t.phase.join(", ")
            };
            let mut meta = format!("   {status} — {phase}");
            if let Some(n) = t.enrollment_count {
                meta.push_str(&format!(" — n={n}"));
            }
            if let Some(s) = &t.lead_sponsor {
                meta.push_str(&format!(" — {s}"));
            }
            println!("{}", meta.dimmed());
            if !t.conditions.is_empty() {
                println!("   {} {}", "conditions:".dimmed(), t.conditions.join(", "));
            }
            println!("   {}", t.url.dimmed());
        }
    }

    for w in &resp.warnings {
        eprintln!("warning [{}/{}]: {}", w.corpus, w.search_mode, w.message);
    }
}
