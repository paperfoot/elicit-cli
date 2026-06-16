use clap::{Parser, Subcommand, ValueEnum};

const HELP_FOOTER: &str = "\
Tips:
  • Run `elicit agent-info | jq` for the full machine-readable capability manifest
  • Set your key once: export ELICIT_API_KEY=elk_live_... (or pass --api-key)
  • Output is JSON when piped; add --json to force it in a terminal
  • `elicit doctor` checks your key and API reachability before real work
  • Reports/reviews are async (5-15 min): add --wait to block, or poll `report get <id>`
  • search/reports need a Pro+ plan; systematic reviews need Enterprise

Examples:
  elicit search \"effects of sleep deprivation on cognition\" --max-results 5
    Semantic paper search, top 5 results

  elicit trials \"semaglutide obesity\" --phase PHASE3 --status RECRUITING
    Phase-3 recruiting clinical trials

  elicit report new \"Do GLP-1 agonists reduce MACE?\" --wait
    Start a research report and block until it finishes

  elicit review get <reviewId> --download csv
    Surface the presigned CSV export URLs for a systematic review";

#[derive(Parser)]
#[command(
    name = "elicit",
    version,
    about = "Agent-grade CLI for the Elicit research API",
    after_long_help = HELP_FOOTER,
)]
pub struct Cli {
    /// Force JSON output even in a terminal
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress informational output
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Elicit API key (overrides ELICIT_API_KEY and config). Prefer the env var.
    #[arg(long, global = true, value_name = "KEY", hide_env_values = true)]
    pub api_key: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

// ── Value enums (clap-validated, mapped to wire strings in command layer) ────

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CorpusArg {
    Elicit,
    Pubmed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeArg {
    Semantic,
    Keyword,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RetractedArg {
    Exclude,
    Include,
    Only,
}

/// Per-search corpus for `review new` (adds clinical_trials over the paper-search corpus).
///
/// The CLI value is the snake_case spec form (`clinical_trials`); the kebab
/// form (`clinical-trials`) is accepted as an alias for forgiveness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewCorpusArg {
    Elicit,
    Pubmed,
    #[value(name = "clinical_trials", alias = "clinical-trials")]
    ClinicalTrials,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceArg {
    Api,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportDownloadArg {
    Pdf,
    Docx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStageArg {
    Search,
    Screen,
    Fulltext,
    Extract,
    Report,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewDownloadArg {
    Csv,
    Xlsx,
    Pdf,
    Docx,
    Txt,
    Bib,
    Ris,
}

// ── Top-level commands ───────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum Commands {
    /// Search 138M+ academic papers (semantic or keyword)
    Search(SearchArgs),

    /// Search clinical trials from ClinicalTrials.gov
    Trials(TrialsArgs),

    /// Create, list, and fetch research reports (async)
    Report {
        #[command(subcommand)]
        action: ReportAction,
    },

    /// Create, list, and fetch systematic reviews (async, Enterprise)
    Review {
        #[command(subcommand)]
        action: ReviewAction,
    },

    /// Validate API key + reachability (offline-safe)
    Doctor,

    /// Machine-readable capability manifest
    #[command(visible_alias = "info")]
    AgentInfo,

    /// Manage skill file installation
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Distribution-aware update check/apply
    Update {
        /// Check only, don't install
        #[arg(long)]
        check: bool,
    },

    /// Hidden: deterministic exit-code trigger for contract tests
    #[command(hide = true)]
    Contract {
        /// Exit code to trigger (0-4)
        code: i32,
    },
}

// ── search ───────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct SearchArgs {
    /// Search query (natural language, or a Lucene expression with --mode keyword)
    pub query: String,

    /// Corpus to search
    #[arg(long, value_enum)]
    pub corpus: Option<CorpusArg>,

    /// Search mode (keyword mode is mutually exclusive with filters)
    #[arg(long, value_enum)]
    pub mode: Option<ModeArg>,

    /// Maximum number of results (1-10000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10000))]
    pub max_results: Option<u32>,

    /// Minimum publication year
    #[arg(long)]
    pub min_year: Option<i32>,

    /// Maximum publication year
    #[arg(long)]
    pub max_year: Option<i32>,

    /// Study type tag (repeatable): Review, Meta-Analysis, Systematic Review, RCT, Longitudinal
    #[arg(long = "type", value_name = "TYPE")]
    pub type_tags: Vec<String>,

    /// Maximum journal quartile (1 = top 25%)
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=4))]
    pub max_quartile: Option<u8>,

    /// Keyword that must appear (repeatable)
    #[arg(long = "include-kw", value_name = "KW")]
    pub include_kw: Vec<String>,

    /// Keyword to exclude (repeatable)
    #[arg(long = "exclude-kw", value_name = "KW")]
    pub exclude_kw: Vec<String>,

    /// Only include papers with an available PDF
    #[arg(long)]
    pub has_pdf: bool,

    /// Only include papers from PubMed
    #[arg(long)]
    pub pubmed_only: bool,

    /// How to handle retracted papers
    #[arg(long, value_enum)]
    pub retracted: Option<RetractedArg>,
}

// ── trials ───────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct TrialsArgs {
    /// Search query (natural language, or a Lucene expression with --mode keyword)
    pub query: String,

    /// Search mode (keyword mode is mutually exclusive with filters)
    #[arg(long, value_enum)]
    pub mode: Option<ModeArg>,

    /// Maximum number of results (1-10000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10000))]
    pub max_results: Option<u32>,

    /// Trial phase (repeatable): NA, EARLY_PHASE1, PHASE1, PHASE2, PHASE3, PHASE4
    #[arg(long, value_name = "PHASE")]
    pub phase: Vec<String>,

    /// Recruitment status (repeatable): RECRUITING, COMPLETED, TERMINATED, ...
    #[arg(long, value_name = "STATUS")]
    pub status: Vec<String>,

    /// Only include trials that have posted results
    #[arg(long)]
    pub has_results: bool,
}

// ── report ───────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ReportAction {
    /// Start a new report (async)
    New(ReportNewArgs),

    /// List your reports
    #[command(visible_alias = "ls")]
    List(ReportListArgs),

    /// Get a report's status and results
    #[command(visible_alias = "show")]
    Get(ReportGetArgs),
}

#[derive(clap::Args)]
pub struct ReportNewArgs {
    /// The research question to investigate
    pub question: String,

    /// Optional report title (otherwise auto-generated)
    #[arg(long)]
    pub title: Option<String>,

    /// Max papers to retrieve during search (default 50, max 1000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub search_papers: Option<u32>,

    /// Max papers in the final extraction table (default 10, max 80)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=80))]
    pub extract_papers: Option<u32>,

    /// Make the report publicly accessible via its URL
    #[arg(long)]
    pub public: bool,

    /// Block and poll until the report completes or fails
    #[arg(long)]
    pub wait: bool,

    /// Seconds between polls when --wait is set (minimum 1)
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub poll_interval: u64,

    /// Give up after this many seconds when --wait is set
    #[arg(long, default_value_t = 1800)]
    pub timeout: u64,
}

#[derive(clap::Args)]
pub struct ReportListArgs {
    /// Filter by status: processing, completed, failed, unknown
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by how the report was created
    #[arg(long, value_enum)]
    pub source: Option<SourceArg>,

    /// Maximum number to return (1-100)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: Option<u32>,

    /// Pagination cursor from a previous nextCursor
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(clap::Args)]
pub struct ReportGetArgs {
    /// Report id (UUID)
    pub id: String,

    /// Include the full report body + abstract
    #[arg(long)]
    pub body: bool,

    /// Block and poll until the report completes or fails
    #[arg(long)]
    pub wait: bool,

    /// Surface the presigned download URL for this format
    #[arg(long, value_enum)]
    pub download: Option<ReportDownloadArg>,

    /// Seconds between polls when --wait is set (minimum 1)
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub poll_interval: u64,

    /// Give up after this many seconds when --wait is set
    #[arg(long, default_value_t = 1800)]
    pub timeout: u64,
}

// ── review ───────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ReviewAction {
    /// Start a new systematic review (async, Enterprise)
    New(ReviewNewArgs),

    /// List your systematic reviews
    #[command(visible_alias = "ls")]
    List(ReviewListArgs),

    /// Get a review's status and results
    #[command(visible_alias = "show")]
    Get(ReviewGetArgs),
}

#[derive(clap::Args)]
pub struct ReviewNewArgs {
    /// The research question the review investigates
    pub question: String,

    /// Optional review title
    #[arg(long)]
    pub title: Option<String>,

    /// Free-form protocol context (PICO, methodology, rationale)
    #[arg(long)]
    pub protocol: Option<String>,

    /// Search query feeding the pipeline (repeatable, max 20)
    #[arg(long = "search", value_name = "QUERY")]
    pub search: Vec<String>,

    /// Corpus for every --search (elicit, pubmed, or clinical_trials; default elicit)
    #[arg(long, value_enum)]
    pub corpus: Option<ReviewCorpusArg>,

    /// Search mode for every --search (default semantic)
    #[arg(long, value_enum)]
    pub mode: Option<ModeArg>,

    /// Max results per --search (1-10000, default 200)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10000))]
    pub max_results: Option<u32>,

    /// Abstract-screening criterion as NAME:INSTRUCTIONS (repeatable)
    #[arg(long = "screen", value_name = "NAME:INSTRUCTIONS")]
    pub screen: Vec<String>,

    /// Fulltext-screening criterion as NAME:INSTRUCTIONS (repeatable; requires --screen or --generate-screening)
    #[arg(long = "fulltext-screen", value_name = "NAME:INSTRUCTIONS")]
    pub fulltext_screen: Vec<String>,

    /// Reuse the abstract-stage criteria at the fulltext stage (requires --screen or --generate-screening)
    #[arg(long)]
    pub reuse_abstract_criteria: bool,

    /// Extraction column as NAME:INSTRUCTIONS or NAME:INSTRUCTIONS:choice1|choice2 (repeatable)
    #[arg(long = "extract-column", value_name = "NAME:INSTRUCTIONS[:c1|c2]")]
    pub extract_column: Vec<String>,

    /// Let Elicit generate additional screening criteria
    #[arg(long)]
    pub generate_screening: bool,

    /// Let Elicit generate additional extraction columns
    #[arg(long)]
    pub generate_extraction: bool,

    /// Allow the model to consult figures during extraction
    #[arg(long)]
    pub use_figures: bool,

    /// Generate a full report at the end (requires extraction columns or --generate-extraction)
    #[arg(long)]
    pub generate_report: bool,

    /// Make the review publicly accessible via its URL
    #[arg(long)]
    pub public: bool,

    /// Block and poll until the review completes or fails
    #[arg(long)]
    pub wait: bool,

    /// Seconds between polls when --wait is set (minimum 1)
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub poll_interval: u64,

    /// Give up after this many seconds when --wait is set
    #[arg(long, default_value_t = 1800)]
    pub timeout: u64,
}

#[derive(clap::Args)]
pub struct ReviewListArgs {
    /// Filter by status: processing, completed, failed, unknown
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by how the review was created
    #[arg(long, value_enum)]
    pub source: Option<SourceArg>,

    /// Maximum number to return (1-100)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: Option<u32>,

    /// Pagination cursor from a previous nextCursor
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(clap::Args)]
pub struct ReviewGetArgs {
    /// Review id (UUID)
    pub id: String,

    /// Block and poll until the review completes or fails
    #[arg(long)]
    pub wait: bool,

    /// Restrict download surfacing to a single stage
    #[arg(long, value_enum)]
    pub stage: Option<ReviewStageArg>,

    /// Surface the presigned download URL for this format
    #[arg(long, value_enum)]
    pub download: Option<ReviewDownloadArg>,

    /// Include the full report body + abstract
    #[arg(long)]
    pub body: bool,

    /// Seconds between polls when --wait is set (minimum 1)
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub poll_interval: u64,

    /// Give up after this many seconds when --wait is set
    #[arg(long, default_value_t = 1800)]
    pub timeout: u64,
}

// ── shared subcommand groups ─────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum SkillAction {
    /// Write skill file to all detected agent platforms
    Install,
    /// Check which platforms have the skill installed
    Status,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Display effective merged configuration
    Show,
    /// Print configuration file path
    Path,
}
