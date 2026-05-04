//! Shared HTTP client builder for the OpenSFHistory Laravel API.
//!
//! Both sharing.rs and opensf_sync.rs need a reqwest client with the same
//! defaults (Accept: application/json + optional Bearer token). This
//! module is the single source of truth so the two stay in sync.

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};

/// Build a reqwest client with sensible defaults for talking to the
/// OpenSFHistory Laravel API. If `token` is provided, every request the
/// returned client makes carries an `Authorization: Bearer <token>`
/// header. Also sets `Accept: application/json` so Laravel returns JSON
/// rather than HTML error pages.
pub fn build_authed_client(token: Option<&str>) -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    if let Some(t) = token {
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", t)) {
            headers.insert(AUTHORIZATION, val);
        }
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Compose a URL from a base + path segments, percent-encoding each segment
/// and handling trailing-slash normalisation for free. Replaces ad-hoc
/// `format!("{}/foo/{}", base, value)` patterns where any unsanitised
/// segment would otherwise need manual encoding.
///
/// Returns the joined Url as a String, or an error if `base` doesn't
/// parse or `cannot_be_a_base` (e.g. `mailto:` or `data:` URLs).
pub fn join_url(base: &str, segments: &[&str]) -> Result<String, String> {
    let mut url = reqwest::Url::parse(base).map_err(|e| format!("Invalid base URL: {}", e))?;
    {
        let mut segs = url
            .path_segments_mut()
            .map_err(|_| "Base URL cannot have path segments".to_string())?;
        segs.pop_if_empty();
        for s in segments {
            segs.push(s);
        }
    }
    Ok(url.into())
}
