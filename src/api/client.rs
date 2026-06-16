//! Blocking HTTP client for the Elicit API.
//!
//! One method per endpoint. Every response runs through `map_status` which
//! turns HTTP status codes into `AppError` variants with the exact exit-code
//! semantics the framework requires:
//!
//!   400            -> InvalidInput            (exit 3)
//!   401 / 403      -> Config                  (exit 2)  [check ELICIT_API_KEY]
//!   402 quota      -> Config                  (exit 2)  [upgrade / wait reset]
//!   404            -> InvalidInput            (exit 3)  [unknown id]
//!   429            -> RateLimited             (exit 4)  [parse Retry-After/Reset]
//!   5xx/timeout/.. -> Transient               (exit 1)
//!
//! The caller is expected to have already resolved the API key (fail-fast,
//! exit 2) BEFORE constructing the client, so no request is ever made without
//! a key.

use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::AppError;

use super::models::*;

/// Rate-limit snapshot parsed from `X-RateLimit-*` response headers. Absent on
/// plans with no daily limit (Enterprise).
#[derive(Debug, Clone, Default, Serialize)]
pub struct RateLimit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<i64>,
}

impl RateLimit {
    fn from_headers(headers: &HeaderMap) -> Self {
        let get = |name: &str| -> Option<i64> {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<i64>().ok())
        };
        Self {
            limit: get("x-ratelimit-limit"),
            remaining: get("x-ratelimit-remaining"),
            reset: get("x-ratelimit-reset"),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.limit.is_none() && self.remaining.is_none() && self.reset.is_none()
    }
}

/// Result of a search, bundling the decoded body with the rate-limit headers so
/// the command layer can surface remaining/reset to the agent.
pub struct SearchOutcome<T> {
    pub body: T,
    pub rate_limit: RateLimit,
}

pub struct ElicitClient {
    http: Client,
    base_url: String,
    api_key: String,
}

/// Default per-request timeout. Searches and create calls are fast; polling
/// uses the same client and re-issues GETs on its own cadence.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

impl ElicitClient {
    /// Build a client. `base_url` must not have a trailing slash; `api_key`
    /// must already be resolved (non-empty).
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!(
                env!("CARGO_BIN_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| {
                AppError::transient_with(
                    format!("failed to build HTTP client: {e}"),
                    "Retry the command",
                )
            })?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn authed(&self, rb: RequestBuilder) -> RequestBuilder {
        let mut headers = HeaderMap::new();
        // Authorization is constructed from the resolved key; HeaderValue
        // construction can only fail on invalid bytes, in which case we fall
        // back to letting the request proceed unauthenticated (the API then
        // returns 401 -> Config, exit 2).
        if let Ok(mut v) = HeaderValue::from_str(&format!("Bearer {}", self.api_key)) {
            v.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, v);
        }
        rb.headers(headers)
    }

    // ── Endpoint 1: POST /search ────────────────────────────────────────────

    pub fn search(
        &self,
        req: &PaperSearchRequest,
    ) -> Result<SearchOutcome<PaperSearchResponse>, AppError> {
        let resp = self
            .authed(self.http.post(self.url("/search")))
            .json(req)
            .send();
        self.decode_with_rate_limit(resp)
    }

    // ── Endpoint 2: POST /search/trials ─────────────────────────────────────

    pub fn search_trials(
        &self,
        req: &TrialSearchRequest,
    ) -> Result<SearchOutcome<TrialSearchResponse>, AppError> {
        let resp = self
            .authed(self.http.post(self.url("/search/trials")))
            .json(req)
            .send();
        self.decode_with_rate_limit(resp)
    }

    // ── Endpoint 3: POST /reports ───────────────────────────────────────────

    pub fn create_report(
        &self,
        req: &CreateReportRequest,
    ) -> Result<CreateReportResponse, AppError> {
        let resp = self
            .authed(self.http.post(self.url("/reports")))
            .json(req)
            .send();
        self.decode(resp)
    }

    // ── Endpoint 4: GET /reports ────────────────────────────────────────────

    pub fn list_reports(&self, query: &[(&str, String)]) -> Result<ListReportsResponse, AppError> {
        let resp = self
            .authed(self.http.get(self.url("/reports")))
            .query(query)
            .send();
        self.decode(resp)
    }

    // ── Endpoint 5: GET /reports/{id} ───────────────────────────────────────

    pub fn get_report(&self, id: &str, include_body: bool) -> Result<GetReportResponse, AppError> {
        let mut rb = self.authed(self.http.get(self.url(&format!("/reports/{id}"))));
        if include_body {
            rb = rb.query(&[("include", "reportBody")]);
        }
        self.decode(rb.send())
    }

    // ── Endpoint 6: POST /systematic-reviews ────────────────────────────────

    pub fn create_review(
        &self,
        req: &CreateReviewRequest,
    ) -> Result<CreateReviewResponse, AppError> {
        let resp = self
            .authed(self.http.post(self.url("/systematic-reviews")))
            .json(req)
            .send();
        self.decode(resp)
    }

    // ── Endpoint 7: GET /systematic-reviews ─────────────────────────────────

    pub fn list_reviews(&self, query: &[(&str, String)]) -> Result<ListReviewsResponse, AppError> {
        let resp = self
            .authed(self.http.get(self.url("/systematic-reviews")))
            .query(query)
            .send();
        self.decode(resp)
    }

    // ── Endpoint 8: GET /systematic-reviews/{id} ────────────────────────────

    pub fn get_review(&self, id: &str, include_body: bool) -> Result<GetReviewResponse, AppError> {
        let mut rb = self.authed(
            self.http
                .get(self.url(&format!("/systematic-reviews/{id}"))),
        );
        if include_body {
            rb = rb.query(&[("include", "reportBody")]);
        }
        self.decode(rb.send())
    }

    // ── Connectivity probe (doctor) ─────────────────────────────────────────

    /// Lightweight reachability check used by `doctor`. Issues a cheap GET
    /// `/reports?limit=1` and reports whether the base URL is reachable plus
    /// the rate-limit headers (which carry plan/quota). Never panics or hangs
    /// beyond the request timeout. Returns Ok even on 4xx (reachable but
    /// unauthorized still proves the endpoint is up); only transport failures
    /// are Err.
    pub fn probe(&self) -> Result<ProbeOutcome, AppError> {
        let resp = self
            .authed(self.http.get(self.url("/reports")))
            .query(&[("limit", "1")])
            .send();
        match resp {
            Ok(r) => {
                let status = r.status();
                let rate_limit = RateLimit::from_headers(r.headers());
                Ok(ProbeOutcome {
                    reachable: true,
                    status: Some(status.as_u16()),
                    rate_limit,
                })
            }
            Err(e) => {
                if e.is_timeout() || e.is_connect() || e.is_request() {
                    Ok(ProbeOutcome {
                        reachable: false,
                        status: None,
                        rate_limit: RateLimit::default(),
                    })
                } else {
                    Err(transport_error(&e))
                }
            }
        }
    }

    // ── Decode helpers ──────────────────────────────────────────────────────

    /// Send result -> typed body. Maps non-2xx to AppError, transport failures
    /// to Transient.
    fn decode<T: DeserializeOwned>(
        &self,
        resp: Result<Response, reqwest::Error>,
    ) -> Result<T, AppError> {
        let response = resp.map_err(|e| transport_error(&e))?;
        let response = check_status(response)?;
        response.json::<T>().map_err(|e| {
            AppError::transient_with(
                format!("failed to decode API response: {e}"),
                "Retry the command; if it persists the API response format may have changed",
            )
        })
    }

    /// Like `decode` but also extracts the rate-limit headers (search paths).
    fn decode_with_rate_limit<T: DeserializeOwned>(
        &self,
        resp: Result<Response, reqwest::Error>,
    ) -> Result<SearchOutcome<T>, AppError> {
        let response = resp.map_err(|e| transport_error(&e))?;
        let response = check_status(response)?;
        let rate_limit = RateLimit::from_headers(response.headers());
        let body = response.json::<T>().map_err(|e| {
            AppError::transient_with(
                format!("failed to decode API response: {e}"),
                "Retry the command; if it persists the API response format may have changed",
            )
        })?;
        Ok(SearchOutcome { body, rate_limit })
    }
}

/// Outcome of `probe`.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeOutcome {
    pub reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "RateLimit::is_empty")]
    pub rate_limit: RateLimit,
}

// ── Status mapping ──────────────────────────────────────────────────────────

/// Inspect the HTTP status. On 2xx, return the response untouched. Otherwise,
/// drain the body to extract the API's `{error:{code,message}}` (best effort)
/// and map to the correct AppError variant + exit code.
fn check_status(response: Response) -> Result<Response, AppError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    // Capture Retry-After / reset before consuming the body (429 path).
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let reset = response
        .headers()
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let api_msg = response
        .json::<ErrorResponse>()
        .ok()
        .map(|e| format!("{} ({})", e.error.message, e.error.code));

    Err(map_status(status, api_msg, retry_after, reset))
}

fn map_status(
    status: StatusCode,
    api_msg: Option<String>,
    retry_after: Option<String>,
    reset: Option<String>,
) -> AppError {
    let code = status.as_u16();
    let detail = api_msg.unwrap_or_else(|| format!("HTTP {code}"));

    match code {
        400 => AppError::invalid_with(
            format!("Elicit rejected the request: {detail}"),
            "Fix the request arguments. Note: keyword search mode and filters are mutually exclusive.",
        ),
        401 | 403 => AppError::config_with(
            format!("authentication/authorization failed: {detail}"),
            "Check ELICIT_API_KEY at https://elicit.com/settings (keys look like elk_live_...). Search and reports need Pro+, systematic reviews need Enterprise.",
        ),
        402 => AppError::config_with(
            format!("insufficient quota: {detail}"),
            "Your plan's workflow quota is exhausted. Upgrade your plan or wait for the next reset at https://elicit.com/settings.",
        ),
        404 => AppError::invalid_with(
            format!("not found: {detail}"),
            "Check the report/review id. It may be invalid, deleted, or owned by another account.",
        ),
        429 => {
            let when = retry_after
                .map(|ra| format!("Retry-After: {ra}s"))
                .or_else(|| reset.map(|r| format!("rate limit resets at epoch {r}")))
                .unwrap_or_else(|| "see X-RateLimit-Reset".to_string());
            AppError::rate_limited_with(
                format!("rate limit exceeded: {detail}"),
                format!("Wait and retry ({when}). Upgrade your plan for higher limits."),
            )
        }
        500..=599 => AppError::transient_with(
            format!("Elicit server error: {detail}"),
            "The Elicit API had a transient error. Retry after a short delay.",
        ),
        _ => AppError::transient_with(
            format!("unexpected HTTP {code}: {detail}"),
            "Retry the command.",
        ),
    }
}

/// Map a reqwest transport error to a Transient AppError (exit 1). Timeouts,
/// connection failures, and decode errors are all retryable.
fn transport_error(e: &reqwest::Error) -> AppError {
    let what = if e.is_timeout() {
        "request timed out"
    } else if e.is_connect() {
        "could not connect to the Elicit API"
    } else {
        "network request failed"
    };
    AppError::transient_with(
        format!("{what}: {e}"),
        "Check your network connection and retry. The Elicit API base URL is configurable via ELICIT_BASE_URL.",
    )
}

// ── Poll helper ─────────────────────────────────────────────────────────────

/// Terminal state of a polled async job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Processing,
    Completed,
    Failed,
    Unknown,
}

impl JobState {
    pub fn from_status(s: &str) -> Self {
        match s {
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "processing" => Self::Processing,
            _ => Self::Unknown,
        }
    }

    /// A state at which polling should stop. `Unknown` is included: the API
    /// uses it for settled-but-unrecognized records, and it never advances, so
    /// continuing to poll it would only spin to the timeout. We surface the
    /// current value to the caller instead.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Unknown)
    }
}

/// Minimum sleep between polls. Guards against `--poll-interval 0`, which would
/// otherwise re-issue GETs as fast as the network allows and torch the daily
/// quota (and likely trigger a 429 storm) for the whole timeout window.
const MIN_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Generic poller for reports and reviews.
///
/// Repeatedly calls `fetch`, extracts `(status, execution_stage)` via
/// `status_of`, and:
///   - returns the final value once status is terminal (`completed`, `failed`,
///     or `unknown` — see `JobState::is_terminal`),
///   - emits `on_stage` whenever the execution stage changes, and to report a
///     tolerated transient blip (commands route this to STDERR in human mode),
///   - sleeps at least `MIN_POLL_INTERVAL` (clamping `interval` up) between
///     polls,
///   - tolerates a one-off Transient fetch error (exit 1: 5xx, connection
///     reset, request timeout) by logging it and retrying on the next tick, so
///     a momentary blip does not discard a 5-15 minute wait; non-transient
///     errors (config/input/rate-limited) still propagate immediately,
///   - gives up with a Transient error after `timeout` elapses.
pub fn poll<T, F, S, P>(
    mut fetch: F,
    status_of: S,
    mut on_stage: P,
    interval: Duration,
    timeout: Duration,
) -> Result<T, AppError>
where
    F: FnMut() -> Result<T, AppError>,
    S: Fn(&T) -> (JobState, Option<String>),
    P: FnMut(&str),
{
    let start = std::time::Instant::now();
    let interval = interval.max(MIN_POLL_INTERVAL);
    let mut last_stage: Option<String> = None;
    let mut last_transient: Option<AppError> = None;

    loop {
        match fetch() {
            Ok(value) => {
                let (state, stage) = status_of(&value);

                if let Some(stage_str) = &stage {
                    if last_stage.as_deref() != Some(stage_str.as_str()) {
                        on_stage(stage_str);
                        last_stage = Some(stage_str.clone());
                    }
                }

                if state.is_terminal() {
                    return Ok(value);
                }
            }
            // Tolerate transient (exit 1) blips; surface everything else now.
            Err(e) if e.exit_code() == 1 => {
                on_stage(&format!("transient error while polling, will retry: {e}"));
                last_transient = Some(e);
            }
            Err(e) => return Err(e),
        }

        if start.elapsed() >= timeout {
            // If the last attempt was a transient failure, prefer reporting that
            // (it is the proximate cause) over a generic still-processing note.
            if let Some(e) = last_transient {
                return Err(e);
            }
            return Err(AppError::transient_with(
                format!(
                    "timed out after {}s waiting for the job to finish (still processing)",
                    timeout.as_secs()
                ),
                "Re-run with a longer --timeout, or poll the get command again later; the job continues server-side.",
            ));
        }

        std::thread::sleep(interval);
    }
}
