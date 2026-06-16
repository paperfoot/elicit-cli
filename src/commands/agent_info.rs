/// Machine-readable capability manifest.
///
/// agent-info is always raw JSON (NOT wrapped in the success envelope) -- it IS
/// the schema definition an agent bootstraps from. Every command listed here is
/// routable in cli.rs, and every flag described exists. Keep this in lockstep
/// with cli.rs: drift is a P0 bug (the agent_info_contract test guards it).
pub fn run() {
    let name = env!("CARGO_PKG_NAME");
    let config_path = crate::config::config_path();

    let info = serde_json::json!({
        "name": name,
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "base_url_default": crate::config::default_base_url(),
        "async_note": "report and review jobs take ~5-15 min; poll the get command (or use --wait) until status is completed or failed. executionStage advances gathering_sources -> screening_abstract -> screening_fulltext -> extracting_data -> generating_report -> done.",
        "plan_note": "search and reports require a Pro+ plan; systematic-reviews require Enterprise. Rate-limit headers (X-RateLimit-Limit/-Remaining/-Reset) are surfaced on search.",
        "commands": {
            "search <query>": {
                "description": "Search 138M+ academic papers (semantic or keyword). Keyword mode is mutually exclusive with filters.",
                "args": [
                    {"name": "query", "kind": "positional", "type": "string", "required": true, "description": "Search query (natural language, or a Lucene expression with --mode keyword)"}
                ],
                "options": [
                    {"name": "--corpus", "type": "string", "required": false, "values": ["elicit", "pubmed"], "description": "Corpus to search (default elicit)"},
                    {"name": "--mode", "type": "string", "required": false, "values": ["semantic", "keyword"], "description": "Search mode (default semantic)"},
                    {"name": "--max-results", "type": "integer", "required": false, "description": "Maximum results (1-10000, default 10)"},
                    {"name": "--min-year", "type": "integer", "required": false, "description": "Minimum publication year"},
                    {"name": "--max-year", "type": "integer", "required": false, "description": "Maximum publication year"},
                    {"name": "--type", "type": "string", "required": false, "repeatable": true, "values": ["Review", "Meta-Analysis", "Systematic Review", "RCT", "Longitudinal"], "description": "Study type tag (repeatable -> typeTags[])"},
                    {"name": "--max-quartile", "type": "integer", "required": false, "values": ["1", "2", "3", "4"], "description": "Maximum journal quartile (1 = top 25%)"},
                    {"name": "--include-kw", "type": "string", "required": false, "repeatable": true, "description": "Keyword that must appear (repeatable)"},
                    {"name": "--exclude-kw", "type": "string", "required": false, "repeatable": true, "description": "Keyword to exclude (repeatable)"},
                    {"name": "--has-pdf", "type": "bool", "required": false, "description": "Only papers with an available PDF"},
                    {"name": "--pubmed-only", "type": "bool", "required": false, "description": "Only papers from PubMed"},
                    {"name": "--retracted", "type": "string", "required": false, "values": ["exclude", "include", "only"], "description": "How to handle retracted papers (default exclude)"}
                ]
            },
            "trials <query>": {
                "description": "Search clinical trials from ClinicalTrials.gov. Keyword mode is mutually exclusive with filters.",
                "args": [
                    {"name": "query", "kind": "positional", "type": "string", "required": true, "description": "Search query (natural language, or a Lucene expression with --mode keyword)"}
                ],
                "options": [
                    {"name": "--mode", "type": "string", "required": false, "values": ["semantic", "keyword"], "description": "Search mode (default semantic)"},
                    {"name": "--max-results", "type": "integer", "required": false, "description": "Maximum results (1-10000, default 10)"},
                    {"name": "--phase", "type": "string", "required": false, "repeatable": true, "values": ["NA", "EARLY_PHASE1", "PHASE1", "PHASE2", "PHASE3", "PHASE4"], "description": "Trial phase (repeatable)"},
                    {"name": "--status", "type": "string", "required": false, "repeatable": true, "values": ["ACTIVE_NOT_RECRUITING", "COMPLETED", "ENROLLING_BY_INVITATION", "NOT_YET_RECRUITING", "RECRUITING", "SUSPENDED", "TERMINATED", "WITHDRAWN", "AVAILABLE"], "description": "Recruitment status (repeatable)"},
                    {"name": "--has-results", "type": "bool", "required": false, "description": "Only trials that have posted results"}
                ]
            },
            "report new <question>": {
                "description": "Start an async research report (Pro+).",
                "args": [
                    {"name": "question", "kind": "positional", "type": "string", "required": true, "description": "The research question to investigate"}
                ],
                "options": [
                    {"name": "--title", "type": "string", "required": false, "description": "Optional report title"},
                    {"name": "--search-papers", "type": "integer", "required": false, "description": "Max papers to retrieve during search (default 50, max 1000)"},
                    {"name": "--extract-papers", "type": "integer", "required": false, "description": "Max papers in the extraction table (default 10, max 80)"},
                    {"name": "--public", "type": "bool", "required": false, "description": "Make the report publicly accessible"},
                    {"name": "--wait", "type": "bool", "required": false, "description": "Block and poll until completed or failed"},
                    {"name": "--poll-interval", "type": "integer", "required": false, "default": 30, "description": "Seconds between polls when --wait is set"},
                    {"name": "--timeout", "type": "integer", "required": false, "default": 1800, "description": "Give up after N seconds when --wait is set"}
                ]
            },
            "report list": {
                "description": "List your reports (newest first).",
                "aliases": ["report ls"],
                "args": [],
                "options": [
                    {"name": "--status", "type": "string", "required": false, "values": ["processing", "completed", "failed", "unknown"], "description": "Filter by status"},
                    {"name": "--source", "type": "string", "required": false, "values": ["api", "user"], "description": "Filter by how the report was created"},
                    {"name": "--limit", "type": "integer", "required": false, "description": "Maximum to return (1-100, default 20)"},
                    {"name": "--cursor", "type": "string", "required": false, "description": "Pagination cursor from a previous nextCursor"}
                ]
            },
            "report get <id>": {
                "description": "Get a report's status and results.",
                "aliases": ["report show"],
                "args": [
                    {"name": "id", "kind": "positional", "type": "string", "required": true, "description": "Report id (UUID)"}
                ],
                "options": [
                    {"name": "--body", "type": "bool", "required": false, "description": "Include the full report body + abstract (include=reportBody)"},
                    {"name": "--wait", "type": "bool", "required": false, "description": "Block and poll until completed or failed"},
                    {"name": "--download", "type": "string", "required": false, "values": ["pdf", "docx"], "description": "Surface the presigned download URL for this format"},
                    {"name": "--poll-interval", "type": "integer", "required": false, "default": 30, "description": "Seconds between polls when --wait is set"},
                    {"name": "--timeout", "type": "integer", "required": false, "default": 1800, "description": "Give up after N seconds when --wait is set"}
                ]
            },
            "review new <question>": {
                "description": "Start an async systematic review (Enterprise).",
                "args": [
                    {"name": "question", "kind": "positional", "type": "string", "required": true, "description": "The research question the review investigates"}
                ],
                "options": [
                    {"name": "--title", "type": "string", "required": false, "description": "Optional review title"},
                    {"name": "--protocol", "type": "string", "required": false, "description": "Free-form protocol context (PICO, methodology, rationale)"},
                    {"name": "--search", "type": "string", "required": false, "repeatable": true, "description": "Search query feeding the pipeline (repeatable, max 20)"},
                    {"name": "--corpus", "type": "string", "required": false, "values": ["elicit", "pubmed", "clinical_trials"], "description": "Corpus applied to every --search (default elicit)"},
                    {"name": "--mode", "type": "string", "required": false, "values": ["semantic", "keyword"], "description": "Search mode applied to every --search (default semantic)"},
                    {"name": "--max-results", "type": "integer", "required": false, "description": "Max results per --search (1-10000, default 200)"},
                    {"name": "--screen", "type": "string", "required": false, "repeatable": true, "format": "NAME:INSTRUCTIONS", "description": "Abstract-screening criterion (repeatable)"},
                    {"name": "--fulltext-screen", "type": "string", "required": false, "repeatable": true, "format": "NAME:INSTRUCTIONS", "description": "Fulltext-screening criterion (repeatable; requires abstract screening)"},
                    {"name": "--reuse-abstract-criteria", "type": "bool", "required": false, "description": "Apply abstract-stage criteria at the fulltext stage (requires abstract screening)"},
                    {"name": "--extract-column", "type": "string", "required": false, "repeatable": true, "format": "NAME:INSTRUCTIONS[:c1|c2]", "description": "Extraction column (repeatable). Append :choice1|choice2 (2-10) for constrained answers"},
                    {"name": "--generate-screening", "type": "bool", "required": false, "description": "Let Elicit generate additional screening criteria"},
                    {"name": "--generate-extraction", "type": "bool", "required": false, "description": "Let Elicit generate additional extraction columns"},
                    {"name": "--use-figures", "type": "bool", "required": false, "description": "Allow the model to consult figures during extraction"},
                    {"name": "--generate-report", "type": "bool", "required": false, "description": "Generate a full report at the end (requires extraction); enables pdf/docx/txt/bib/ris exports"},
                    {"name": "--public", "type": "bool", "required": false, "description": "Make the review publicly accessible"},
                    {"name": "--wait", "type": "bool", "required": false, "description": "Block and poll until completed or failed"},
                    {"name": "--poll-interval", "type": "integer", "required": false, "default": 30, "description": "Seconds between polls when --wait is set (minimum 1)"},
                    {"name": "--timeout", "type": "integer", "required": false, "default": 1800, "description": "Give up after N seconds when --wait is set"}
                ]
            },
            "review list": {
                "description": "List your systematic reviews (newest first).",
                "aliases": ["review ls"],
                "args": [],
                "options": [
                    {"name": "--status", "type": "string", "required": false, "values": ["processing", "completed", "failed", "unknown"], "description": "Filter by status"},
                    {"name": "--source", "type": "string", "required": false, "values": ["api", "user"], "description": "Filter by how the review was created"},
                    {"name": "--limit", "type": "integer", "required": false, "description": "Maximum to return (1-100, default 20)"},
                    {"name": "--cursor", "type": "string", "required": false, "description": "Pagination cursor from a previous nextCursor"}
                ]
            },
            "review get <id>": {
                "description": "Get a review's status, exports, and report.",
                "aliases": ["review show"],
                "args": [
                    {"name": "id", "kind": "positional", "type": "string", "required": true, "description": "Review id (UUID)"}
                ],
                "options": [
                    {"name": "--wait", "type": "bool", "required": false, "description": "Block and poll until completed or failed"},
                    {"name": "--stage", "type": "string", "required": false, "values": ["search", "screen", "fulltext", "extract", "report"], "description": "Restrict download surfacing to a single stage"},
                    {"name": "--download", "type": "string", "required": false, "values": ["csv", "xlsx", "pdf", "docx", "txt", "bib", "ris"], "description": "Surface the presigned download URL for this format"},
                    {"name": "--body", "type": "bool", "required": false, "description": "Include the full report body + abstract (include=reportBody)"},
                    {"name": "--poll-interval", "type": "integer", "required": false, "default": 30, "description": "Seconds between polls when --wait is set"},
                    {"name": "--timeout", "type": "integer", "required": false, "default": 1800, "description": "Give up after N seconds when --wait is set"}
                ]
            },
            "doctor": {
                "description": "Validate API key presence + elk_ format, base-URL reachability, and surface plan/quota. Offline-safe: exits 2 with no key, never hangs.",
                "args": [],
                "options": []
            },
            "agent-info": {
                "description": "This manifest (raw JSON, not enveloped).",
                "aliases": ["info"],
                "args": [],
                "options": []
            },
            "skill install": {
                "description": "Install skill file to agent platforms (Claude, Codex, Gemini).",
                "args": [],
                "options": []
            },
            "skill status": {
                "description": "Check skill installation status.",
                "args": [],
                "options": []
            },
            "config show": {
                "description": "Display effective merged configuration (API key masked).",
                "args": [],
                "options": []
            },
            "config path": {
                "description": "Show configuration file path.",
                "args": [],
                "options": []
            },
            "update": {
                "description": "Distribution-aware update check/apply. `--check` is always safe and exits 0 even when the release feed is unreachable (status=check_failed).",
                "args": [],
                "options": [
                    {"name": "--check", "type": "bool", "required": false, "default": false, "description": "Check only, don't install (always exits 0)"}
                ],
                "install_sources": [
                    "standalone", "homebrew", "cargo", "cargo_binstall", "npm",
                    "bun", "uv_tool", "pipx", "winget", "scoop", "apt", "managed", "unknown"
                ],
                "data_fields": [
                    "current_version", "latest_version", "status", "install_source",
                    "update_mode", "upgrade_command", "release_url", "requires_skill_reinstall"
                ]
            }
        },
        "global_flags": {
            "--json": {"description": "Force JSON output (auto-enabled when piped)", "type": "bool", "default": false},
            "--quiet": {"description": "Suppress informational output", "type": "bool", "default": false},
            "--api-key": {"description": "Elicit API key (overrides ELICIT_API_KEY and config). Prefer the env var.", "type": "string", "default": null}
        },
        "exit_codes": {
            "0": "Success",
            "1": "Transient error (network, 5xx, timeout) -- retry",
            "2": "Config error (missing/invalid key 401/403, quota 402) -- fix setup",
            "3": "Bad input (invalid args, 400, unknown id 404) -- fix arguments",
            "4": "Rate limited (429) -- wait and retry"
        },
        "envelope": {
            "version": "1",
            "success": "{ version, status, data }",
            "error": "{ version, status, error: { code, message, suggestion } }"
        },
        "config": {
            "path": config_path.display().to_string(),
            "env_prefix": "ELICIT_",
            "api_key_env": "ELICIT_API_KEY",
            "api_key_resolution": ["--api-key flag", "ELICIT_API_KEY env", "config keys.api_key"],
            "base_url_env": "ELICIT_BASE_URL"
        },
        "auto_json_when_piped": true
    });
    println!("{}", serde_json::to_string_pretty(&info).unwrap());
}
