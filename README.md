<div align="center">

# elicit

**The complete command-line client for the [Elicit](https://elicit.com) research API.**

Search 125M+ academic papers and clinical trials, run AI research reports, and drive full systematic reviews — from your terminal, as JSON, with exit codes an AI agent can branch on.

<br />

[![Star this repo](https://img.shields.io/github/stars/paperfoot/elicit-cli?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/paperfoot/elicit-cli/stargazers)
&nbsp;&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

<br />

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MSRV 1.85+](https://img.shields.io/badge/MSRV-1.85%2B-orange?style=for-the-badge)](https://www.rust-lang.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![Built on agent-cli-framework](https://img.shields.io/badge/built_on-agent--cli--framework-6e40c9?style=for-the-badge)](https://github.com/paperfoot/agent-cli-framework)

---

Literature search, evidence reports, and systematic reviews from one Rust binary. No MCP server, no Python, no separate docs to drift — the binary on your PATH is the whole interface, for you at a terminal and for any AI agent that can shell out.

[Install](#install) · [Setup](#setup) · [Commands](#commands) · [Exit codes](#exit-codes) · [JSON envelope](#json-envelope) · [For agents](#for-agents)

</div>

---

## Why this exists

Elicit ships an official example CLI. It covers 3 of the 8 API endpoints and turns every failure — bad key, exhausted quota, rate limit — into a bare `exit 1`. An agent calling it can't tell "fix your key" from "wait and retry," and you can't pull a finished report's text or run a systematic review at all.

`elicit` closes that gap. Every endpoint, every filter, and an error contract an agent can act on.

| | Elicit's official `elicit.py` | **`elicit`** (this repo) |
|---|---|---|
| API coverage | 3 of 8 endpoints | **all 8** — paper search, trial search, reports, systematic reviews |
| Errors | everything → `exit 1` | **semantic exit codes**: bad key → `2`, rate limit → `4`, bad input → `3` |
| Output | raw API JSON or plain text | **versioned JSON envelope**, auto-detected when piped |
| Full report text | not retrievable | `report get --body` |
| Clinical trials | absent | first-class `trials` command |
| Systematic reviews | absent | screening criteria + extraction columns + exports |
| Self-description | none | `agent-info` manifest + `doctor` diagnostics |
| Runtime | Python + `requests` | one static Rust binary, sub-10ms cold start |

---

## Install

```bash
cargo install --path .              # from this repo
cargo install --locked elicit       # from crates.io (once published)
brew install paperfoot/tap/elicit   # Homebrew (once published)
```

Then check it works:

```bash
elicit --version
elicit doctor          # checks your key + API reachability (offline-safe)
```

---

## Setup

Get an API key from <https://elicit.com/settings> (keys look like `elk_live_...`). The CLI reads it from the first source that has it:

1. `--api-key <KEY>` flag
2. `ELICIT_API_KEY` environment variable
3. `keys.api_key` in the config file

```bash
export ELICIT_API_KEY=elk_live_your_key_here
elicit doctor          # api_key: pass, api_reachable: pass
```

Your key is never printed in plain text — `config show` and `doctor` mask it to `elk_...1234`.

**Plan tiers:** paper/trial search and reports need **Pro+**; systematic reviews need **Enterprise**. Reports and reviews are asynchronous (~5–15 min) — poll with `get` or block with `--wait`.

<details>
<summary>Optional config file</summary>

Defaults work with no config file. To persist settings, edit the path shown by `elicit config path` (macOS: `~/Library/Application Support/elicit/config.toml`):

```toml
base_url = "https://elicit.com/api/v1"   # also overridable via ELICIT_BASE_URL

[keys]
# Prefer the ELICIT_API_KEY env var over storing the key here.
# api_key = "elk_live_..."

[update]
enabled = true
install_source = "auto"
```
</details>

---

## Commands

Every command takes the global flags `--json` (force JSON in a terminal), `--quiet` (drop human progress; JSON still emits), and `--api-key <KEY>`. Output is a colored table in a terminal and **JSON when piped**. Data goes to **stdout**, errors and progress to **stderr**, so `elicit ... | jq` never breaks.

### `search` — 125M+ academic papers

```bash
elicit search "effects of sleep deprivation on cognition" --max-results 5
elicit search "CRISPR base editing" --corpus pubmed --min-year 2020 --type RCT --type Meta-Analysis
elicit search "rapamycin longevity" --has-pdf --max-quartile 1 --exclude-kw mice
```

| Flag | Values | Notes |
|------|--------|-------|
| `--corpus` | `elicit` \| `pubmed` | default `elicit` |
| `--mode` | `semantic` \| `keyword` | default `semantic`; keyword mode excludes filters |
| `--max-results` | `1`–`10000` | default `10` |
| `--min-year` / `--max-year` | year | publication-year bounds |
| `--type` | `Review`, `Meta-Analysis`, `Systematic Review`, `RCT`, `Longitudinal` | repeatable |
| `--max-quartile` | `1`–`4` | `1` = top 25% of journals |
| `--include-kw` / `--exclude-kw` | keyword | repeatable |
| `--has-pdf` / `--pubmed-only` | flag | narrow the corpus |
| `--retracted` | `exclude` \| `include` \| `only` | default `exclude` |

### `trials` — clinical trials

```bash
elicit trials "semaglutide obesity" --phase PHASE3 --status RECRUITING
elicit trials "CAR-T lymphoma" --has-results --max-results 20
```

`--mode`, `--max-results`, `--phase` (`NA`/`EARLY_PHASE1`/`PHASE1`–`PHASE4`, repeatable), `--status` (`RECRUITING`, `COMPLETED`, `TERMINATED`, … repeatable), `--has-results`.

### `report` — AI research reports (async, Pro+)

```bash
elicit report new "Do GLP-1 agonists reduce major adverse cardiac events?" --wait
elicit report list --status completed --limit 10        # alias: ls
elicit report get <reportId> --body                     # alias: show; full report text
elicit report get <reportId> --download pdf             # presigned PDF/DOCX URL
```

`report new`: `--title`, `--search-papers N` (default 50), `--extract-papers N` (default 10), `--public`, `--wait`, `--poll-interval S`, `--timeout S`.
`report get`: `--body`, `--wait`, `--download pdf|docx`.

### `review` — systematic reviews (async, Enterprise)

The signature Elicit workflow: searches → abstract/full-text screening → data-extraction columns → synthesized report.

```bash
elicit review new "Does metformin extend healthy lifespan in non-diabetics?" \
  --search "metformin lifespan" --search "metformin aging RCT" --corpus pubmed --max-results 300 \
  --screen "Human study:Must be conducted in human subjects" \
  --fulltext-screen "Outcome reported:Reports a longevity or healthspan outcome" --reuse-abstract-criteria \
  --extract-column "Sample size:Report the total N enrolled" \
  --extract-column "Benefit:Did the intervention help?:yes|no|unclear" \
  --generate-extraction --use-figures --generate-report --wait

elicit review get <reviewId> --download csv --stage extract     # stage export
elicit review get <reviewId> --download pdf                     # final report (needs --generate-report)
```

`review new` adds `--protocol`, repeatable `--search` (max 20), `--screen`/`--fulltext-screen` (`NAME:INSTRUCTIONS`), `--extract-column` (append `:c1|c2` for a constrained answer set, 2–10 choices), `--generate-screening`, `--generate-extraction`, `--use-figures`, `--generate-report`. `review get` mirrors `report get` plus `--stage` and the full export set (`csv|xlsx|pdf|docx|txt|bib|ris`).

### `doctor` and built-ins

```bash
elicit doctor            # key + reachability + plan/quota; exits 2 with no key, never hangs
elicit agent-info        # machine-readable capability manifest (alias: info)
elicit skill install     # register the skill with Claude / Codex / Gemini
elicit config show       # effective config, key masked
elicit update --check    # distribution-aware update check
```

---

## Exit codes

The whole point — an agent branches on these without parsing a single line of text.

| Code | Meaning | Triggered by | Agent action |
|------|---------|--------------|--------------|
| `0` | Success | — | Continue |
| `1` | Transient | `5xx`, timeout, connect/decode failure | Retry with backoff |
| `2` | Config | missing key (before any request), `401`/`403`, `402` quota | Fix setup, don't retry blindly |
| `3` | Bad input | invalid args, `400`, `404` unknown id | Fix arguments |
| `4` | Rate limited | `429` | Wait per `Retry-After` / `X-RateLimit-Reset`, then retry |

`--help` and `--version` always exit `0`. Every API command resolves the key **first** and fails fast with exit `2` if it's missing — no request leaves your machine without one.

---

## JSON envelope

Piped or `--json`, every command emits a versioned envelope. Data on stdout, errors on stderr.

```jsonc
// success (stdout)
{ "version": "1", "status": "success", "data": { "papers": [ /* ... */ ], "warnings": [] } }

// error (stderr)
{ "version": "1", "status": "error",
  "error": { "code": "config_error",
             "message": "no Elicit API key found",
             "suggestion": "Set ELICIT_API_KEY or check config with: elicit config show" } }
```

```bash
# titles of the top 5 papers
elicit search "telomere length aging" --max-results 5 | jq -r '.data.papers[].title'

# branch on quota vs rate-limit in a script
elicit search "foo" >out.json 2>err.json
case $? in
  0) jq '.data.papers | length' out.json ;;
  2) echo "key/quota:";  jq -r '.error.suggestion' err.json ;;
  4) echo "rate limit:"; jq -r '.error.suggestion' err.json ;;
esac
```

---

## For agents

Start with the manifest — it lists every command with full argument schemas, the exit-code contract, the envelope shape, and the config block:

```bash
elicit agent-info | jq
```

`agent-info` is a tested contract: every command it lists is routable, every flag it names exists. Install the bundled skill so Claude Code, Codex, and Gemini discover the tool on their own:

```bash
elicit skill install
```

It triggers on *search papers, literature search, find studies, find clinical trials, research report, systematic review, "what does the research say."*

---

## Built on agent-cli-framework

`elicit` follows the [agent-cli-framework](https://github.com/paperfoot/agent-cli-framework) patterns — `agent-info` discovery, JSON envelopes, semantic exit codes, `doctor`, skill self-install, and distribution-aware updates. Learn one CLI built this way and you've learned them all.

## Contributing

Issues and PRs welcome. `cargo test` runs the full contract suite (80 tests covering exit codes, envelope shape, and `agent-info` routability). Keep `agent-info` in sync with `cli.rs` — that's the one invariant.

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">

Built by [Boris Djordjevic](https://github.com/longevityboris) at [Paperfoot AI](https://paperfoot.com)

<br />

**If this saved you time:**

[![Star this repo](https://img.shields.io/github/stars/paperfoot/elicit-cli?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/paperfoot/elicit-cli/stargazers)
&nbsp;&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

</div>
