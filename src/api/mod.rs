//! Elicit API layer: typed models + a blocking HTTP client.
//!
//! `models` holds serde structs for every request/response shape in the
//! OpenAPI spec. `client` wraps a blocking reqwest client with one method per
//! endpoint, the HTTP-status -> AppError mapping, rate-limit header parsing,
//! and a generic poll helper for the async report/review jobs.

pub mod client;
pub mod models;

pub use client::{ElicitClient, JobState, RateLimit, poll};
