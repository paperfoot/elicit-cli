use std::time::Duration;

use crate::api::models::*;
use crate::api::{ElicitClient, JobState, poll};
use crate::cli::{ReportAction, ReportDownloadArg, ReportGetArgs, ReportListArgs, ReportNewArgs};
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

pub fn run(
    ctx: Ctx,
    action: ReportAction,
    api_key: Option<&str>,
    config: &AppConfig,
) -> Result<(), AppError> {
    let key = config::resolve_api_key(api_key, config)?;
    let client = ElicitClient::new(&config.base_url, &key)?;

    match action {
        ReportAction::New(args) => new(ctx, &client, args),
        ReportAction::List(args) => list(ctx, &client, args),
        ReportAction::Get(args) => get(ctx, &client, args),
    }
}

// ── report new ───────────────────────────────────────────────────────────────

fn new(ctx: Ctx, client: &ElicitClient, args: ReportNewArgs) -> Result<(), AppError> {
    if args.question.trim().is_empty() {
        return Err(AppError::invalid_with(
            "research question cannot be empty",
            "Provide a non-empty research question as the first argument",
        ));
    }

    let req = CreateReportRequest {
        research_question: args.question.clone(),
        title: args.title.clone(),
        max_search_papers: args.search_papers,
        max_extract_papers: args.extract_papers,
        is_public: if args.public { Some(true) } else { None },
    };

    let created = client.create_report(&req)?;

    if args.wait {
        eprintln_human(
            ctx,
            &format!("report {} accepted; polling...", created.report_id),
        );
        let final_report = poll_report(
            ctx,
            client,
            &created.report_id,
            false,
            Duration::from_secs(args.poll_interval),
            Duration::from_secs(args.timeout),
        )?;
        output::print_success_or(ctx, &final_report, render_get_human);
    } else {
        output::print_success_or(ctx, &created, |c| {
            use owo_colors::OwoColorize;
            println!("{} {}", "report:".green(), c.report_id.bold());
            println!("   status: {}", c.status);
            println!("   {}", c.url.dimmed());
            println!(
                "   {}",
                format!("poll with: elicit report get {}", c.report_id).dimmed()
            );
        });
    }

    Ok(())
}

// ── report list ──────────────────────────────────────────────────────────────

fn list(ctx: Ctx, client: &ElicitClient, args: ReportListArgs) -> Result<(), AppError> {
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

    let resp = client.list_reports(&q)?;

    output::print_success_or(ctx, &resp, |r| {
        use owo_colors::OwoColorize;
        if r.reports.is_empty() {
            println!("{}", "No reports.".dimmed());
            return;
        }
        let mut table = comfy_table::Table::new();
        table.set_header(vec![
            "Report ID",
            "Status",
            "Stage",
            "Source",
            "Created",
            "Title",
        ]);
        for item in &r.reports {
            table.add_row(vec![
                item.report_id.clone(),
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

// ── report get ───────────────────────────────────────────────────────────────

fn get(ctx: Ctx, client: &ElicitClient, args: ReportGetArgs) -> Result<(), AppError> {
    let report = if args.wait {
        poll_report(
            ctx,
            client,
            &args.id,
            args.body,
            Duration::from_secs(args.poll_interval),
            Duration::from_secs(args.timeout),
        )?
    } else {
        client.get_report(&args.id, args.body)?
    };

    if let Some(fmt) = args.download {
        return surface_download(ctx, &report, fmt);
    }

    output::print_success_or(ctx, &report, render_get_human);
    Ok(())
}

fn surface_download(
    ctx: Ctx,
    report: &GetReportResponse,
    fmt: ReportDownloadArg,
) -> Result<(), AppError> {
    let url = match fmt {
        ReportDownloadArg::Pdf => report.pdf_url.as_deref(),
        ReportDownloadArg::Docx => report.docx_url.as_deref(),
    };
    match url {
        Some(u) => {
            let data = serde_json::json!({
                "reportId": report.report_id,
                "format": format!("{fmt:?}").to_lowercase(),
                "url": u,
            });
            output::print_success_or(ctx, &data, |_| {
                println!("{u}");
            });
            Ok(())
        }
        None => Err(AppError::invalid_with(
            format!(
                "no {} download URL available (status: {})",
                format!("{fmt:?}").to_lowercase(),
                report.status
            ),
            "Downloads appear only once the report status is completed; re-fetch after completion (URLs expire after 7 days).",
        )),
    }
}

// ── shared polling + rendering ───────────────────────────────────────────────

fn poll_report(
    ctx: Ctx,
    client: &ElicitClient,
    id: &str,
    include_body: bool,
    interval: Duration,
    timeout: Duration,
) -> Result<GetReportResponse, AppError> {
    poll(
        || client.get_report(id, include_body),
        |r| (JobState::from_status(&r.status), r.execution_stage.clone()),
        |stage| eprintln_human(ctx, &format!("stage: {stage}")),
        interval,
        timeout,
    )
}

fn render_get_human(report: &GetReportResponse) {
    use owo_colors::OwoColorize;
    println!("{} {}", "report:".green(), report.report_id.bold());
    println!("   status: {}", report.status);
    if let Some(stage) = &report.execution_stage {
        println!("   stage: {stage}");
    }
    println!("   {}", report.url.dimmed());

    if let Some(result) = &report.result {
        println!();
        println!("{}", result.title.bold());
        println!("{}", result.summary);
        if let Some(body) = &result.report_body {
            println!();
            println!("{body}");
        }
    }
    if let Some(err) = &report.error {
        eprintln!("error: {} ({})", err.message, err.code);
    }
    if report.pdf_url.is_some() || report.docx_url.is_some() {
        println!();
        if let Some(u) = &report.pdf_url {
            println!("   {} {u}", "pdf:".dimmed());
        }
        if let Some(u) = &report.docx_url {
            println!("   {} {u}", "docx:".dimmed());
        }
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

pub fn source_str(src: crate::cli::SourceArg) -> &'static str {
    match src {
        crate::cli::SourceArg::Api => "api",
        crate::cli::SourceArg::User => "user",
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

/// Print an informational line to STDERR in human (non-quiet) mode only.
fn eprintln_human(ctx: Ctx, msg: &str) {
    if ctx.quiet {
        return;
    }
    if let crate::output::Format::Human = ctx.format {
        use owo_colors::OwoColorize;
        eprintln!("{}", msg.dimmed());
    }
}
