// In-app bug reporting to GitHub Issues (Debugging settings tab).
//
// Any logged-in user can file a report; the dialog collects a category and
// a free-text description, and this module turns that into a GitHub issue
// via the REST API. The fine-grained token and target repo live in
// app_settings (`github_issues_token` is a SECRET_KEYS entry — admin-only
// reads via get_setting), but submission reads them directly from the DB so
// editors can file reports without being able to see the token.

use serde::{Deserialize, Serialize};

/// Issues land here when `github_issues_repo` is unset.
pub const DEFAULT_REPO: &str = "Western-Neighborhoods-Project/WNP-Image-Archive-desktop-app";

/// Max length of a derived issue title, in characters.
const MAX_TITLE_CHARS: usize = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportCategory {
    Bug,
    Feature,
    Idea,
    Data,
    Ux,
}

impl ReportCategory {
    /// GitHub label applied to the issue.
    pub fn label(&self) -> &'static str {
        match self {
            ReportCategory::Bug => "bug",
            ReportCategory::Feature => "enhancement",
            ReportCategory::Idea => "idea",
            ReportCategory::Data => "data",
            ReportCategory::Ux => "ux",
        }
    }

    /// Human-readable name, used in the issue body (labels can be dropped
    /// by GitHub when the token lacks push access, so the body always
    /// states the category too).
    pub fn display_name(&self) -> &'static str {
        match self {
            ReportCategory::Bug => "Bug",
            ReportCategory::Feature => "Feature request",
            ReportCategory::Idea => "Idea",
            ReportCategory::Data => "Data issue",
            ReportCategory::Ux => "UX / Polish",
        }
    }
}

/// Environment/context appended to every issue body.
pub struct ReportContext {
    pub app_version: String,
    pub os: String,
    pub view: String,
    pub username: String,
}

/// Derive an issue title from the description: first non-empty line,
/// truncated to `MAX_TITLE_CHARS` characters with an ellipsis.
pub fn derive_title(description: &str) -> String {
    let first_line = description
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if first_line.chars().count() <= MAX_TITLE_CHARS {
        return first_line.to_string();
    }
    let truncated: String = first_line.chars().take(MAX_TITLE_CHARS - 1).collect();
    format!("{}…", truncated)
}

/// Full issue body: the description, a separator, then the context block.
pub fn compose_body(description: &str, category: ReportCategory, ctx: &ReportContext) -> String {
    format!(
        "{}\n\n---\n**Category:** {}\n**App version:** {}\n**OS:** {}\n**View:** {}\n**Reported by:** {}",
        description.trim(),
        category.display_name(),
        ctx.app_version,
        ctx.os,
        ctx.view,
        ctx.username,
    )
}

/// JSON payload for POST /repos/{owner}/{repo}/issues.
pub fn build_issue_payload(
    description: &str,
    category: ReportCategory,
    ctx: &ReportContext,
) -> serde_json::Value {
    serde_json::json!({
        "title": derive_title(description),
        "body": compose_body(description, category, ctx),
        "labels": [category.label()],
    })
}

/// Turn a non-201 GitHub API response into a message the report dialog can
/// show. `api_message` is the `message` field GitHub returns in error bodies.
pub fn describe_github_failure(status: u16, api_message: Option<&str>) -> String {
    match status {
        401 => "GitHub rejected the token (401). Check it in Settings → Debugging.".to_string(),
        404 => "Repository not found (404). Check the repo name in Settings → Debugging, and \
                that the token has Issues read/write access to it."
            .to_string(),
        410 => "Issues are disabled for this repository (410).".to_string(),
        _ => match api_message {
            Some(m) => format!("GitHub returned {}: {}", status, m),
            None => format!("GitHub returned {}", status),
        },
    }
}

/// Split an "owner/repo" setting into its parts. Rejects anything that
/// isn't exactly two non-empty slash-separated segments.
pub fn parse_repo(setting: &str) -> Option<(String, String)> {
    let mut parts = setting.trim().split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(repo), None) if !owner.is_empty() && !repo.is_empty() => {
            Some((owner.to_string(), repo.to_string()))
        }
        _ => None,
    }
}

// ── Command ────────────────────────────────────────────────────────────────

/// Created-issue info handed back to the dialog for the success state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedIssue {
    pub number: i64,
    pub html_url: String,
}

/// File a report as a GitHub issue. Any logged-in user; the token is read
/// straight from the DB (not via get_setting) so editors can submit without
/// admin-level access to the secret itself.
#[tauri::command]
pub async fn submit_bug_report(
    category: ReportCategory,
    description: String,
    current_view: Option<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::db::AppState>,
) -> Result<SubmittedIssue, String> {
    let session = crate::auth::require_session(&state)?;
    let description = description.trim().to_string();
    if description.is_empty() {
        return Err("Please describe what happened".to_string());
    }

    // Read settings in a scope so the DB lock is never held across an await.
    let (enabled, token, repo_setting) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> Option<String> {
            db.query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                rusqlite::params![key],
                |r| r.get::<_, String>(0),
            )
            .ok()
        };
        (
            get("debug_reporting_enabled"),
            get("github_issues_token"),
            get("github_issues_repo"),
        )
    };

    if enabled.as_deref() != Some("true") {
        return Err("Debug reporting is turned off (Settings → Debugging)".to_string());
    }
    let token = token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or("No GitHub token configured. An admin can add one in Settings → Debugging.")?;
    let repo_setting = repo_setting
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| DEFAULT_REPO.to_string());
    let (owner, repo) = parse_repo(&repo_setting).ok_or_else(|| {
        format!(
            "Invalid GitHub repo '{}' — expected owner/repo (Settings → Debugging)",
            repo_setting
        )
    })?;

    let ctx = ReportContext {
        app_version: app.package_info().version.to_string(),
        os: format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH),
        view: current_view
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "unknown".to_string()),
        username: session.username,
    };
    let payload = build_issue_payload(&description, category, &ctx);

    // Reuse the shared client for Bearer auth + timeouts; GitHub's Accept
    // header and mandatory User-Agent are set per-request.
    let client = crate::http::build_authed_client(Some(&token));
    let url = crate::http::join_url("https://api.github.com", &["repos", &owner, &repo, "issues"])?;
    let resp = client
        .post(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", format!("WNP-Image-Archive/{}", ctx.app_version))
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {}", e))?;

    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    if status == 201 {
        Ok(SubmittedIssue {
            number: body["number"].as_i64().unwrap_or(0),
            html_url: body["html_url"].as_str().unwrap_or_default().to_string(),
        })
    } else {
        Err(describe_github_failure(status, body["message"].as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ReportContext {
        ReportContext {
            app_version: "0.6.0".to_string(),
            os: "macOS".to_string(),
            view: "library".to_string(),
            username: "daniel".to_string(),
        }
    }

    // ── derive_title ───────────────────────────────────────────────────

    #[test]
    fn title_is_first_line() {
        assert_eq!(
            derive_title("Thumbnails vanish on rescan\nSteps: open library…"),
            "Thumbnails vanish on rescan"
        );
    }

    #[test]
    fn title_skips_leading_blank_lines_and_trims() {
        assert_eq!(derive_title("\n\n  spaced out  \nmore"), "spaced out");
    }

    #[test]
    fn title_truncates_long_first_line_with_ellipsis() {
        let long = "x".repeat(100);
        let title = derive_title(&long);
        assert_eq!(title.chars().count(), MAX_TITLE_CHARS);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn title_truncation_respects_char_boundaries() {
        // Multi-byte chars: byte-index truncation would panic or split a char.
        let long = "é".repeat(100);
        let title = derive_title(&long);
        assert_eq!(title.chars().count(), MAX_TITLE_CHARS);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn short_title_is_untouched() {
        assert_eq!(derive_title("just right"), "just right");
    }

    // ── category mapping ───────────────────────────────────────────────

    #[test]
    fn categories_map_to_github_labels() {
        assert_eq!(ReportCategory::Bug.label(), "bug");
        assert_eq!(ReportCategory::Feature.label(), "enhancement");
        assert_eq!(ReportCategory::Idea.label(), "idea");
        assert_eq!(ReportCategory::Data.label(), "data");
        assert_eq!(ReportCategory::Ux.label(), "ux");
    }

    #[test]
    fn categories_have_display_names() {
        assert_eq!(ReportCategory::Bug.display_name(), "Bug");
        assert_eq!(ReportCategory::Feature.display_name(), "Feature request");
        assert_eq!(ReportCategory::Idea.display_name(), "Idea");
        assert_eq!(ReportCategory::Data.display_name(), "Data issue");
        assert_eq!(ReportCategory::Ux.display_name(), "UX / Polish");
    }

    #[test]
    fn category_deserializes_from_lowercase() {
        let c: ReportCategory = serde_json::from_str("\"data\"").unwrap();
        assert_eq!(c, ReportCategory::Data);
    }

    // ── compose_body ───────────────────────────────────────────────────

    #[test]
    fn body_contains_description_and_context() {
        let body = compose_body("It broke.", ReportCategory::Bug, &ctx());
        assert!(body.starts_with("It broke."));
        assert!(body.contains("**Category:** Bug"));
        assert!(body.contains("**App version:** 0.6.0"));
        assert!(body.contains("**OS:** macOS"));
        assert!(body.contains("**View:** library"));
        assert!(body.contains("**Reported by:** daniel"));
    }

    #[test]
    fn body_separates_description_from_context() {
        let body = compose_body("Text", ReportCategory::Idea, &ctx());
        assert!(body.contains("\n\n---\n"));
    }

    // ── build_issue_payload ────────────────────────────────────────────

    #[test]
    fn payload_has_title_body_and_label() {
        let payload = build_issue_payload("Broken thing\ndetails", ReportCategory::Data, &ctx());
        assert_eq!(payload["title"], "Broken thing");
        assert_eq!(payload["labels"], serde_json::json!(["data"]));
        let body = payload["body"].as_str().unwrap();
        assert!(body.contains("details"));
        assert!(body.contains("**Category:** Data issue"));
    }

    // ── describe_github_failure ────────────────────────────────────────

    #[test]
    fn bad_token_points_at_settings() {
        let msg = describe_github_failure(401, Some("Bad credentials"));
        assert!(msg.contains("token"));
        assert!(msg.contains("Settings"));
    }

    #[test]
    fn missing_repo_mentions_repo_and_access() {
        let msg = describe_github_failure(404, Some("Not Found"));
        assert!(msg.contains("Repository"));
        assert!(msg.contains("token"));
    }

    #[test]
    fn other_failures_surface_status_and_github_message() {
        let msg = describe_github_failure(422, Some("Validation Failed"));
        assert!(msg.contains("422"));
        assert!(msg.contains("Validation Failed"));
    }

    #[test]
    fn failure_without_message_still_reports_status() {
        let msg = describe_github_failure(500, None);
        assert!(msg.contains("500"));
    }

    // ── parse_repo ─────────────────────────────────────────────────────

    #[test]
    fn default_repo_is_the_canonical_org_repo() {
        // The repo moved from danielucas/… to the org. GitHub 301-redirects
        // the old path, and reqwest turns a redirected POST into a GET, so
        // pointing at the old name would silently break issue creation.
        assert_eq!(
            DEFAULT_REPO,
            "Western-Neighborhoods-Project/WNP-Image-Archive-desktop-app"
        );
        assert!(parse_repo(DEFAULT_REPO).is_some());
    }

    #[test]
    fn parses_owner_slash_repo() {
        assert_eq!(
            parse_repo("Western-Neighborhoods-Project/WNP-Image-Archive-desktop-app"),
            Some((
                "Western-Neighborhoods-Project".to_string(),
                "WNP-Image-Archive-desktop-app".to_string()
            ))
        );
    }

    #[test]
    fn parse_repo_trims_whitespace() {
        assert_eq!(
            parse_repo("  owner/repo  "),
            Some(("owner".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn parse_repo_rejects_malformed() {
        assert_eq!(parse_repo("no-slash"), None);
        assert_eq!(parse_repo("/repo"), None);
        assert_eq!(parse_repo("owner/"), None);
        assert_eq!(parse_repo("a/b/c"), None);
        assert_eq!(parse_repo(""), None);
    }
}
