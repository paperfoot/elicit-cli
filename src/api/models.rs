//! Serde models for every Elicit API type.
//!
//! Verified field-by-field against /tmp/elicit-openapi.json (OpenAPI 3.1).
//!
//! Conventions:
//!   - `#[serde(rename_all = "camelCase")]` on every type (the API is camelCase).
//!   - Nullable response fields are `Option<T>` (the spec marks them `["T","null"]`).
//!   - Request bodies use `skip_serializing_if = "Option::is_none"` so unset
//!     fields are omitted entirely rather than sent as `null`.
//!   - Enums (corpus / searchMode / retracted / etc.) are serialized as the
//!     exact wire strings the API expects.

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════════════
// Shared error envelope (response)
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

// ════════════════════════════════════════════════════════════════════════════
// POST /search  -- PaperSearchRequest / PaperSearchResponse
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperSearchRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<SearchMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corpus: Option<Corpus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<PaperFilters>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_epoch_s: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_epoch_s: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_quartile: Option<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_keywords: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_keywords: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub type_tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_pdf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubmed_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retracted: Option<Retracted>,
}

impl PaperFilters {
    /// True when every field is at its empty default, i.e. nothing to send.
    pub fn is_empty(&self) -> bool {
        self.min_year.is_none()
            && self.max_year.is_none()
            && self.min_epoch_s.is_none()
            && self.max_epoch_s.is_none()
            && self.max_quartile.is_none()
            && self.include_keywords.is_empty()
            && self.exclude_keywords.is_empty()
            && self.type_tags.is_empty()
            && self.has_pdf.is_none()
            && self.pubmed_only.is_none()
            && self.retracted.is_none()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaperSearchResponse {
    pub papers: Vec<Paper>,
    #[serde(default)]
    pub warnings: Vec<SearchWarning>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paper {
    pub elicit_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub year: Option<i32>,
    #[serde(rename = "abstract")]
    pub abstract_: Option<String>,
    pub doi: Option<String>,
    pub pmid: Option<String>,
    pub venue: Option<String>,
    pub cited_by_count: Option<i64>,
    #[serde(default)]
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchWarning {
    pub corpus: String,
    pub search_mode: String,
    pub message: String,
    #[serde(default)]
    pub warning_details: Option<SearchWarningDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchWarningDetails {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub messages: Vec<String>,
}

// ════════════════════════════════════════════════════════════════════════════
// POST /search/trials  -- TrialSearchRequest / TrialSearchResponse
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialSearchRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<SearchMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_filters: Option<TrialFilters>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialFilters {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub phase: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recruitment_status: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_results: Option<bool>,
}

impl TrialFilters {
    pub fn is_empty(&self) -> bool {
        self.phase.is_empty() && self.recruitment_status.is_empty() && self.has_results.is_none()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrialSearchResponse {
    pub trials: Vec<Trial>,
    #[serde(default)]
    pub warnings: Vec<SearchWarning>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trial {
    pub nct_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub url: String,
    pub overall_status: Option<String>,
    #[serde(default)]
    pub phase: Vec<String>,
    pub study_type: Option<String>,
    pub enrollment_count: Option<i64>,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub interventions: Vec<String>,
    pub lead_sponsor: Option<String>,
    pub start_date: Option<String>,
    pub primary_completion_date: Option<String>,
    pub completion_date: Option<String>,
    pub has_results: Option<bool>,
    pub last_updated_year: Option<i32>,
}

// ════════════════════════════════════════════════════════════════════════════
// POST /reports  -- CreateReportRequest / CreateReportResponse
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportRequest {
    pub research_question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_search_papers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_extract_papers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportResponse {
    pub report_id: String,
    pub status: String,
    pub url: String,
    pub is_public: bool,
}

// ── GET /reports (list) ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReportsResponse {
    pub reports: Vec<ReportListItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportListItem {
    pub report_id: String,
    pub status: String,
    pub execution_stage: Option<String>,
    pub title: String,
    pub url: String,
    pub source: String,
    pub created_at: String,
    pub is_public: bool,
}

// ── GET /reports/{id} ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetReportResponse {
    pub report_id: String,
    pub status: String,
    pub execution_stage: Option<String>,
    pub url: String,
    pub is_public: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ReportResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiErrorBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docx_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResult {
    pub title: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_body: Option<String>,
    #[serde(rename = "abstract", default, skip_serializing_if = "Option::is_none")]
    pub abstract_: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════════
// POST /systematic-reviews  -- CreateSystematicReviewRequest / response
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReviewRequest {
    pub research_question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_report: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub searches: Vec<ReviewSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_screening: Option<ScreeningStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fulltext_screening: Option<FulltextScreeningStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction: Option<ExtractionStage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSearch {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corpus: Option<ReviewCorpus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<SearchMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreeningStage {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<Criterion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FulltextScreeningStage {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<Criterion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reuse_abstract_criteria: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Criterion {
    pub name: String,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionStage {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<ExtractionQuestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_figures: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionQuestion {
    pub name: String,
    pub instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReviewResponse {
    pub review_id: String,
    pub status: String,
    pub url: String,
    pub is_public: bool,
}

// ── GET /systematic-reviews (list) ───────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReviewsResponse {
    pub reviews: Vec<ReviewListItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewListItem {
    pub review_id: String,
    pub status: String,
    pub execution_stage: Option<String>,
    pub title: String,
    pub url: String,
    pub source: String,
    pub created_at: String,
    pub is_public: bool,
}

// ── GET /systematic-reviews/{id} ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetReviewResponse {
    pub review_id: String,
    pub status: String,
    pub execution_stage: Option<String>,
    pub url: String,
    pub is_public: bool,
    pub data_freshness: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<ReviewData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiErrorBody>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<StageData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<StageData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fulltext: Option<StageData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<StageData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<ReportData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageData {
    pub csv: String,
    pub xlsx: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportData {
    pub result: ReviewResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docx: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub txt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bib: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ris: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResult {
    pub title: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_body: Option<String>,
    #[serde(rename = "abstract", default, skip_serializing_if = "Option::is_none")]
    pub abstract_: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════════
// Enums (exact wire strings)
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Semantic,
    Keyword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Corpus {
    Elicit,
    Pubmed,
}

// Variant names intentionally mirror the API's `*_retracted` vocabulary.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Retracted {
    ExcludeRetracted,
    IncludeRetracted,
    OnlyRetracted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewCorpus {
    Elicit,
    Pubmed,
    ClinicalTrials,
}

#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn paper_request_camel_case_and_omits_none() {
        let req = PaperSearchRequest {
            query: "x".into(),
            search_mode: Some(SearchMode::Keyword),
            max_results: Some(5),
            corpus: Some(Corpus::Pubmed),
            filters: None,
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["query"], "x");
        assert_eq!(v["searchMode"], "keyword");
        assert_eq!(v["maxResults"], 5);
        assert_eq!(v["corpus"], "pubmed");
        assert!(v.get("filters").is_none(), "None filters must be omitted");
    }

    #[test]
    fn filters_wire_names_and_enums() {
        let f = PaperFilters {
            min_year: Some(2020),
            max_epoch_s: Some(1672531200),
            type_tags: vec!["RCT".into(), "Meta-Analysis".into()],
            has_pdf: Some(true),
            retracted: Some(Retracted::OnlyRetracted),
            ..Default::default()
        };
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v["minYear"], 2020);
        assert_eq!(v["maxEpochS"], 1672531200_i64);
        assert_eq!(v["typeTags"][0], "RCT");
        assert_eq!(v["hasPdf"], true);
        assert_eq!(v["retracted"], "only_retracted");
        // empty vecs / none omitted
        assert!(v.get("includeKeywords").is_none());
        assert!(v.get("maxYear").is_none());
    }

    #[test]
    fn review_corpus_clinical_trials_wire() {
        let s = ReviewSearch {
            query: "q".into(),
            corpus: Some(ReviewCorpus::ClinicalTrials),
            search_mode: None,
            max_results: None,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["corpus"], "clinical_trials");
        assert!(v.get("searchMode").is_none());
    }

    #[test]
    fn review_request_wires_generate_report_and_fulltext_and_choices() {
        let req = CreateReviewRequest {
            research_question: "rq".into(),
            title: None,
            protocol_details: None,
            is_public: None,
            generate_report: Some(true),
            searches: vec![ReviewSearch {
                query: "q".into(),
                corpus: Some(ReviewCorpus::Pubmed),
                search_mode: Some(SearchMode::Keyword),
                max_results: Some(300),
            }],
            abstract_screening: Some(ScreeningStage {
                criteria: vec![Criterion {
                    name: "Human".into(),
                    instructions: "In humans".into(),
                }],
                generate: None,
            }),
            fulltext_screening: Some(FulltextScreeningStage {
                criteria: vec![],
                reuse_abstract_criteria: Some(true),
            }),
            extraction: Some(ExtractionStage {
                questions: vec![ExtractionQuestion {
                    name: "Benefit".into(),
                    instructions: "Did it help?".into(),
                    choices: Some(vec!["yes".into(), "no".into()]),
                }],
                generate: None,
                use_figures: None,
            }),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["generateReport"], true);
        assert_eq!(v["searches"][0]["corpus"], "pubmed");
        assert_eq!(v["searches"][0]["searchMode"], "keyword");
        assert_eq!(v["searches"][0]["maxResults"], 300);
        assert_eq!(v["fulltextScreening"]["reuseAbstractCriteria"], true);
        // empty fulltext criteria vec is omitted
        assert!(v["fulltextScreening"].get("criteria").is_none());
        assert_eq!(v["extraction"]["questions"][0]["choices"][0], "yes");
        assert_eq!(v["extraction"]["questions"][0]["choices"][1], "no");
    }

    #[test]
    fn paper_abstract_field_roundtrips() {
        let json = serde_json::json!({
            "elicitId": "a", "title": "t", "authors": ["x"], "year": 2023,
            "abstract": "the abstract", "doi": null, "pmid": null, "venue": null,
            "citedByCount": 42, "urls": []
        });
        let p: Paper = serde_json::from_value(json).unwrap();
        assert_eq!(p.abstract_.as_deref(), Some("the abstract"));
        assert_eq!(p.cited_by_count, Some(42));
        assert_eq!(p.elicit_id.as_deref(), Some("a"));
    }

    #[test]
    fn report_get_parses_completed() {
        let json = serde_json::json!({
            "reportId": "r", "status": "completed", "executionStage": "done",
            "url": "u", "isPublic": false,
            "result": {"title": "T", "summary": "S"},
            "pdfUrl": "https://s3/pdf"
        });
        let r: GetReportResponse = serde_json::from_value(json).unwrap();
        assert_eq!(r.status, "completed");
        assert_eq!(r.execution_stage.as_deref(), Some("done"));
        assert_eq!(r.result.unwrap().title, "T");
        assert_eq!(r.pdf_url.as_deref(), Some("https://s3/pdf"));
    }

    #[test]
    fn create_report_request_defaults_omitted() {
        let req = CreateReportRequest {
            research_question: "q".into(),
            title: None,
            max_search_papers: None,
            max_extract_papers: None,
            is_public: None,
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["researchQuestion"], "q");
        assert_eq!(v.as_object().unwrap().len(), 1, "only researchQuestion should serialize");
    }
}
