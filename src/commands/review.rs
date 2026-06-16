use std::time::Duration;

use crate::api::models::*;
use crate::api::{ElicitClient, JobState, poll};
use crate::cli::{
    ReviewAction, ReviewCorpusArg, ReviewDownloadArg, ReviewGetArgs, ReviewListArgs, ReviewNewArgs,
    ReviewStageArg,
};
use crate::commands::report::{source_str, truncate};
use crate::commands::search::map_mode;
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

pub fn run(ctx: Ctx, action: ReviewAction, api_key: Option<&str>, config: &AppConfig) -> Result<(), AppError> {
    let key = config::resolve_api_key(api_key, config)?;
    let client = ElicitClient::new(&config.base_url, &key)?;

    match action {
        ReviewAction::New(args) => new(ctx, &client, args),
        ReviewAction::List(args) => list(ctx, &client, args),
        ReviewAction::Get(args) => get(ctx, &client, args),
    }
}

// ── review new ───────────────────────────────────────────────────────────────

fn new(ctx: Ctx, client: &ElicitClient, args: ReviewNewArgs) -> Result<(), AppError> {
    if args.question.trim().is_empty() {
        return Err(AppError::invalid_with(
            "research question cannot be empty",
            "Provide a non-empty research question as the first argument",
        ));
    }

    // searches[]: one ReviewSearch per --search; the review-level --corpus /
    // --mode / --max-results apply to every search. API defaults to a semantic
    // search on the research question when omitted.
    let corpus = args.corpus.map(map_review_corpus);
    let mode = args.mode.map(map_mode);
    let searches: Vec<ReviewSearch> = args
        .search
        .iter()
        .map(|q| ReviewSearch {
            query: q.clone(),
            corpus,
            search_mode: mode,
            max_results: args.max_results,
        })
        .collect();

    // abstractScreening: present if any criteria provided OR --generate-screening.
    let screen_criteria = parse_criteria(&args.screen, "--screen")?;
    let has_abstract_screening = !screen_criteria.is_empty() || args.generate_screening;
    let abstract_screening = if has_abstract_screening {
        Some(ScreeningStage {
            criteria: screen_criteria,
            generate: if args.generate_screening { Some(true) } else { None },
        })
    } else {
        None
    };

    // fulltextScreening: present if any fulltext criteria provided OR
    // --reuse-abstract-criteria. Per spec it requires abstractScreening to be
    // present; guard locally for a clear exit-3 error.
    let fulltext_criteria = parse_criteria(&args.fulltext_screen, "--fulltext-screen")?;
    let has_fulltext = !fulltext_criteria.is_empty() || args.reuse_abstract_criteria;
    if has_fulltext && !has_abstract_screening {
        return Err(AppError::invalid_with(
            "fulltext screening requires abstract screening",
            "Add at least one --screen NAME:INSTRUCTIONS (or --generate-screening) before using --fulltext-screen / --reuse-abstract-criteria.",
        ));
    }
    let fulltext_screening = if has_fulltext {
        Some(FulltextScreeningStage {
            criteria: fulltext_criteria,
            reuse_abstract_criteria: if args.reuse_abstract_criteria {
                Some(true)
            } else {
                None
            },
        })
    } else {
        None
    };

    // extraction: present if any columns provided OR --generate-extraction.
    let extraction_questions = parse_extraction(&args.extract_column)?;
    let has_extraction = !extraction_questions.is_empty() || args.generate_extraction;
    let extraction = if has_extraction {
        Some(ExtractionStage {
            questions: extraction_questions,
            generate: if args.generate_extraction { Some(true) } else { None },
            use_figures: if args.use_figures { Some(true) } else { None },
        })
    } else {
        None
    };

    // generateReport requires extraction (spec). Guard locally so the agent gets
    // a followable exit-3 error instead of a 400 round-trip.
    if args.generate_report && !has_extraction {
        return Err(AppError::invalid_with(
            "--generate-report requires extraction",
            "Add at least one --extract-column NAME:INSTRUCTIONS (or --generate-extraction) so the review has data to report on.",
        ));
    }

    let req = CreateReviewRequest {
        research_question: args.question.clone(),
        title: args.title.clone(),
        protocol_details: args.protocol.clone(),
        is_public: if args.public { Some(true) } else { None },
        generate_report: if args.generate_report { Some(true) } else { None },
        searches,
        abstract_screening,
        fulltext_screening,
        extraction,
    };

    let created = client.create_review(&req)?;

    if args.wait {
        eprintln_human(ctx, &format!("review {} accepted; polling...", created.review_id));
        let final_review = poll_review(
            ctx,
            client,
            &created.review_id,
            false,
            Duration::from_secs(args.poll_interval),
            Duration::from_secs(args.timeout),
        )?;
        output::print_success_or(ctx, &final_review, render_get_human);
    } else {
        output::print_success_or(ctx, &created, |c| {
            use owo_colors::OwoColorize;
            println!("{} {}", "review:".green(), c.review_id.bold());
            println!("   status: {}", c.status);
            println!("   {}", c.url.dimmed());
            println!(
                "   {}",
                format!("poll with: elicit review get {}", c.review_id).dimmed()
            );
        });
    }

    Ok(())
}

// ── review list ──────────────────────────────────────────────────────────────

fn list(ctx: Ctx, client: &ElicitClient, args: ReviewListArgs) -> Result<(), AppError> {
    let mut q: Vec<(&str, String)> = Vec::new();
    if let Some(s) = &args.status {
        q.push(("status", s.clone()));
    }
    if let Some(src) = args.source {
        q.push(("source", source_str(src).into()));
    }
    if let Some(l) = args.limit {
        q.push(("limit", l.to_string()));
    }
    if let Some(c) = &args.cursor {
        q.push(("cursor", c.clone()));
    }

    let resp = client.list_reviews(&q)?;

    output::print_success_or(ctx, &resp, |r| {
        use owo_colors::OwoColorize;
        if r.reviews.is_empty() {
            println!("{}", "No reviews.".dimmed());
            return;
        }
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Review ID", "Status", "Stage", "Source", "Created", "Title"]);
        for item in &r.reviews {
            table.add_row(vec![
                item.review_id.clone(),
                item.status.clone(),
                item.execution_stage.clone().unwrap_or_default(),
                item.source.clone(),
                item.created_at.clone(),
                truncate(&item.title, 50),
            ]);
        }
        println!("{table}");
        if let Some(cursor) = &r.next_cursor {
            println!("{}", format!("next cursor: {cursor}").dimmed());
        }
    });

    Ok(())
}

// ── review get ───────────────────────────────────────────────────────────────

fn get(ctx: Ctx, client: &ElicitClient, args: ReviewGetArgs) -> Result<(), AppError> {
    let review = if args.wait {
        poll_review(
            ctx,
            client,
            &args.id,
            args.body,
            Duration::from_secs(args.poll_interval),
            Duration::from_secs(args.timeout),
        )?
    } else {
        client.get_review(&args.id, args.body)?
    };

    if let Some(fmt) = args.download {
        return surface_download(ctx, &review, args.stage, fmt);
    }

    output::print_success_or(ctx, &review, render_get_human);
    Ok(())
}

fn surface_download(
    ctx: Ctx,
    review: &GetReviewResponse,
    stage: Option<ReviewStageArg>,
    fmt: ReviewDownloadArg,
) -> Result<(), AppError> {
    let data = review.data.as_ref().ok_or_else(|| {
        AppError::invalid_with(
            format!("no exports available yet (status: {})", review.status),
            "Exports appear as each stage completes; poll `review get <id>` again, or add --wait.",
        )
    })?;

    // Collect (stage, url) candidates matching the requested format, optionally
    // filtered to a single stage.
    let mut found: Vec<(&str, String)> = Vec::new();
    let want = |s: ReviewStageArg| stage.is_none() || stage == Some(s);

    match fmt {
        ReviewDownloadArg::Csv | ReviewDownloadArg::Xlsx => {
            let pick = |sd: &StageData| -> String {
                match fmt {
                    ReviewDownloadArg::Csv => sd.csv.clone(),
                    _ => sd.xlsx.clone(),
                }
            };
            if want(ReviewStageArg::Search) {
                if let Some(sd) = &data.search {
                    found.push(("search", pick(sd)));
                }
            }
            if want(ReviewStageArg::Screen) {
                if let Some(sd) = &data.screen {
                    found.push(("screen", pick(sd)));
                }
            }
            if want(ReviewStageArg::Fulltext) {
                if let Some(sd) = &data.fulltext {
                    found.push(("fulltext", pick(sd)));
                }
            }
            if want(ReviewStageArg::Extract) {
                if let Some(sd) = &data.extract {
                    found.push(("extract", pick(sd)));
                }
            }
        }
        ReviewDownloadArg::Pdf
        | ReviewDownloadArg::Docx
        | ReviewDownloadArg::Txt
        | ReviewDownloadArg::Bib
        | ReviewDownloadArg::Ris => {
            if want(ReviewStageArg::Report) {
                if let Some(rd) = &data.report {
                    let url = match fmt {
                        ReviewDownloadArg::Pdf => rd.pdf.clone(),
                        ReviewDownloadArg::Docx => rd.docx.clone(),
                        ReviewDownloadArg::Txt => rd.txt.clone(),
                        ReviewDownloadArg::Bib => rd.bib.clone(),
                        ReviewDownloadArg::Ris => rd.ris.clone(),
                        _ => None,
                    };
                    if let Some(u) = url {
                        found.push(("report", u));
                    }
                }
            }
        }
    }

    if found.is_empty() {
        return Err(AppError::invalid_with(
            format!(
                "no {} export available for the requested stage (status: {})",
                fmt_str(fmt),
                review.status
            ),
            "Check `review get <id>` for which stages have data. Report-format exports (pdf/docx/txt/bib/ris) require the review to have been created with --generate-report.",
        ));
    }

    let payload: Vec<serde_json::Value> = found
        .iter()
        .map(|(stage, url)| {
            serde_json::json!({ "stage": stage, "format": fmt_str(fmt), "url": url })
        })
        .collect();

    output::print_success_or(ctx, &payload, |rows| {
        for row in rows {
            if let Some(u) = row.get("url").and_then(|v| v.as_str()) {
                println!("{u}");
            }
        }
    });
    Ok(())
}

// ── shared polling + rendering ───────────────────────────────────────────────

fn poll_review(
    ctx: Ctx,
    client: &ElicitClient,
    id: &str,
    include_body: bool,
    interval: Duration,
    timeout: Duration,
) -> Result<GetReviewResponse, AppError> {
    poll(
        || client.get_review(id, include_body),
        |r| (JobState::from_status(&r.status), r.execution_stage.clone()),
        |stage| eprintln_human(ctx, &format!("stage: {stage}")),
        interval,
        timeout,
    )
}

fn render_get_human(review: &GetReviewResponse) {
    use owo_colors::OwoColorize;
    println!("{} {}", "review:".green(), review.review_id.bold());
    println!("   status: {}", review.status);
    if let Some(stage) = &review.execution_stage {
        println!("   stage: {stage}");
    }
    println!("   {}", review.url.dimmed());

    if let Some(data) = &review.data {
        println!();
        println!("{}", "available exports:".bold());
        if data.search.is_some() {
            println!("   search:   csv, xlsx");
        }
        if data.screen.is_some() {
            println!("   screen:   csv, xlsx");
        }
        if data.fulltext.is_some() {
            println!("   fulltext: csv, xlsx");
        }
        if data.extract.is_some() {
            println!("   extract:  csv, xlsx");
        }
        if let Some(rd) = &data.report {
            let mut formats = vec!["json"];
            if rd.pdf.is_some() {
                formats.push("pdf");
            }
            if rd.docx.is_some() {
                formats.push("docx");
            }
            if rd.txt.is_some() {
                formats.push("txt");
            }
            if rd.bib.is_some() {
                formats.push("bib");
            }
            if rd.ris.is_some() {
                formats.push("ris");
            }
            println!("   report:   {}", formats.join(", "));
            println!();
            println!("{}", rd.result.title.bold());
            println!("{}", rd.result.summary);
            if let Some(body) = &rd.result.report_body {
                println!();
                println!("{body}");
            }
        }
    }

    if let Some(err) = &review.error {
        eprintln!("error: {} ({})", err.message, err.code);
    }
}

// ── NAME:INSTRUCTIONS parsing ────────────────────────────────────────────────

fn parse_criteria(raw: &[String], flag: &str) -> Result<Vec<Criterion>, AppError> {
    raw.iter()
        .map(|s| {
            let (name, instructions) = split_pair(s, flag)?;
            Ok(Criterion { name, instructions })
        })
        .collect()
}

fn parse_extraction(raw: &[String]) -> Result<Vec<ExtractionQuestion>, AppError> {
    raw.iter()
        .map(|s| {
            // Grammar: NAME:INSTRUCTIONS[:choice1|choice2|...]. The first colon
            // separates NAME from the rest; an OPTIONAL second colon introduces
            // a pipe-delimited fixed choice list (2-10 items per the spec).
            let (name, rest) = s.split_once(':').ok_or_else(|| extract_err(s))?;
            let name = name.trim();
            if name.is_empty() {
                return Err(extract_err(s));
            }
            let (instructions, choices) = match rest.split_once(':') {
                Some((instr, choices_raw)) => {
                    let choices: Vec<String> = choices_raw
                        .split('|')
                        .map(|c| c.trim().to_string())
                        .filter(|c| !c.is_empty())
                        .collect();
                    if choices.len() < 2 || choices.len() > 10 {
                        return Err(AppError::invalid_with(
                            format!(
                                "invalid --extract-column choices in '{s}' (need 2-10 pipe-separated values, got {})",
                                choices.len()
                            ),
                            "Use --extract-column \"Outcome:Did the drug help?:yes|no|maybe\" (2-10 choices).",
                        ));
                    }
                    (instr.trim(), Some(choices))
                }
                None => (rest.trim(), None),
            };
            if instructions.is_empty() {
                return Err(extract_err(s));
            }
            Ok(ExtractionQuestion {
                name: name.to_string(),
                instructions: instructions.to_string(),
                choices,
            })
        })
        .collect()
}

fn extract_err(s: &str) -> AppError {
    AppError::invalid_with(
        format!("invalid --extract-column value '{s}' (expected NAME:INSTRUCTIONS[:c1|c2])"),
        "Use --extract-column \"Sample size:Report the N\" or add choices: \"Human study:In humans?:yes|no\"",
    )
}

/// Split a `NAME:INSTRUCTIONS` argument on the first colon.
fn split_pair(s: &str, flag: &str) -> Result<(String, String), AppError> {
    match s.split_once(':') {
        Some((name, instructions)) if !name.trim().is_empty() && !instructions.trim().is_empty() => {
            Ok((name.trim().to_string(), instructions.trim().to_string()))
        }
        _ => Err(AppError::invalid_with(
            format!("invalid {flag} value '{s}' (expected NAME:INSTRUCTIONS)"),
            format!("Use {flag} \"Human study:The study must be in human subjects\""),
        )),
    }
}

fn map_review_corpus(c: ReviewCorpusArg) -> ReviewCorpus {
    match c {
        ReviewCorpusArg::Elicit => ReviewCorpus::Elicit,
        ReviewCorpusArg::Pubmed => ReviewCorpus::Pubmed,
        ReviewCorpusArg::ClinicalTrials => ReviewCorpus::ClinicalTrials,
    }
}

fn fmt_str(fmt: ReviewDownloadArg) -> &'static str {
    match fmt {
        ReviewDownloadArg::Csv => "csv",
        ReviewDownloadArg::Xlsx => "xlsx",
        ReviewDownloadArg::Pdf => "pdf",
        ReviewDownloadArg::Docx => "docx",
        ReviewDownloadArg::Txt => "txt",
        ReviewDownloadArg::Bib => "bib",
        ReviewDownloadArg::Ris => "ris",
    }
}

fn eprintln_human(ctx: Ctx, msg: &str) {
    if ctx.quiet {
        return;
    }
    if let crate::output::Format::Human = ctx.format {
        use owo_colors::OwoColorize;
        eprintln!("{}", msg.dimmed());
    }
}
