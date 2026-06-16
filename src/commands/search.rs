use crate::api::ElicitClient;
use crate::api::models::*;
use crate::cli::{CorpusArg, ModeArg, RetractedArg, SearchArgs};
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

pub fn run(
    ctx: Ctx,
    args: SearchArgs,
    api_key: Option<&str>,
    config: &AppConfig,
) -> Result<(), AppError> {
    // Fail fast (exit 2) before any network call.
    let key = config::resolve_api_key(api_key, config)?;

    if args.query.trim().is_empty() {
        return Err(AppError::invalid_with(
            "query cannot be empty",
            "Provide a non-empty search query as the first argument",
        ));
    }

    let mode = args.mode.map(map_mode);

    // Build filters from the flags.
    let mut filters = PaperFilters {
        min_year: args.min_year,
        max_year: args.max_year,
        max_quartile: args.max_quartile,
        include_keywords: args.include_kw.clone(),
        exclude_keywords: args.exclude_kw.clone(),
        type_tags: args.type_tags.clone(),
        retracted: args.retracted.map(map_retracted),
        ..Default::default()
    };
    if args.has_pdf {
        filters.has_pdf = Some(true);
    }
    if args.pubmed_only {
        filters.pubmed_only = Some(true);
    }

    // Keyword mode and filters are mutually exclusive (API returns 400). Guard
    // locally with a clearer message + exit 3.
    if mode == Some(SearchMode::Keyword) && !filters.is_empty() {
        return Err(AppError::invalid_with(
            "keyword search mode cannot be combined with filters",
            "Use --mode semantic with filters, or move filter expressions into the query string and drop the filter flags",
        ));
    }

    let req = PaperSearchRequest {
        query: args.query.clone(),
        search_mode: mode,
        max_results: args.max_results,
        corpus: args.corpus.map(map_corpus),
        filters: if filters.is_empty() {
            None
        } else {
            Some(filters)
        },
    };

    let client = ElicitClient::new(&config.base_url, &key)?;
    let outcome = client.search(&req)?;

    output::emit_rate_limit(ctx, &outcome.rate_limit);

    let body = outcome.body;
    output::print_success_or(ctx, &body, render_human);

    Ok(())
}

fn render_human(resp: &PaperSearchResponse) {
    use owo_colors::OwoColorize;

    if resp.papers.is_empty() {
        println!("{}", "No papers found.".dimmed());
    } else {
        for (i, p) in resp.papers.iter().enumerate() {
            let year = p
                .year
                .map(|y| y.to_string())
                .unwrap_or_else(|| "n.d.".into());
            println!("{} {}", format!("{}.", i + 1).dimmed(), p.title.bold());
            let authors = if p.authors.is_empty() {
                "Unknown authors".to_string()
            } else if p.authors.len() > 3 {
                format!("{}, et al.", p.authors[..3].join(", "))
            } else {
                p.authors.join(", ")
            };
            let mut meta = format!("   {authors} ({year})");
            if let Some(v) = &p.venue {
                meta.push_str(&format!(" — {v}"));
            }
            if let Some(c) = p.cited_by_count {
                meta.push_str(&format!(" — {c} citations"));
            }
            println!("{}", meta.dimmed());
            if let Some(doi) = &p.doi {
                println!("   {} https://doi.org/{doi}", "doi:".dimmed());
            }
        }
    }

    for w in &resp.warnings {
        eprintln!("warning [{}/{}]: {}", w.corpus, w.search_mode, w.message);
    }
}

// ── enum mapping (CLI arg -> wire) ───────────────────────────────────────────

pub fn map_mode(m: ModeArg) -> SearchMode {
    match m {
        ModeArg::Semantic => SearchMode::Semantic,
        ModeArg::Keyword => SearchMode::Keyword,
    }
}

pub fn map_corpus(c: CorpusArg) -> Corpus {
    match c {
        CorpusArg::Elicit => Corpus::Elicit,
        CorpusArg::Pubmed => Corpus::Pubmed,
    }
}

pub fn map_retracted(r: RetractedArg) -> Retracted {
    match r {
        RetractedArg::Exclude => Retracted::ExcludeRetracted,
        RetractedArg::Include => Retracted::IncludeRetracted,
        RetractedArg::Only => Retracted::OnlyRetracted,
    }
}
