//! Pilot consumer provisioner (pilot plan Task 4).
//!
//! `provisionPilotConsumer(spaceId, templateVersion)` runs a minimal closed
//! loop: create repo via GitHub API -> render `templates/pilot-consumer/`
//! (substituting the four `__PILOT_*` placeholders) -> push a scaffold branch
//! via the Git Data API (no git CLI, no `github.com` git port) -> open the
//! initial PR. If any step after repo creation fails, the repo is
//! deleted again (fail-closed rollback).
//!
//! Live provisioning needs Task 3 credentials: either a static token
//! (`PILOT_GITHUB_TOKEN` or `GITHUB_TOKEN`) or a GitHub App
//! (`PILOT_GITHUB_APP_ID` + `PILOT_GITHUB_APP_KEY`), plus `PILOT_GITHUB_ORG`;
//! without them the mutation fails fast before creating anything.

use std::path::{Path, PathBuf};

use business_architecture::application::space_service::SpaceService;
use business_architecture::infrastructure::persistence::space_audit_repo::SeaOrmAuditLogRepo;
use business_architecture::infrastructure::persistence::space_repo::{
    SeaOrmMembershipRepo, SeaOrmSpaceRepo,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PLACEHOLDER_REPO: &str = "__PILOT_REPO__";
pub const PLACEHOLDER_RUNNER: &str = "__PILOT_RUNNER__";
pub const PLACEHOLDER_FRONTEND_URL: &str = "__PILOT_FRONTEND_URL__";
pub const PLACEHOLDER_API_URL: &str = "__PILOT_API_URL__";

/// GitHub App used for pilot provisioning when no static token is set.
pub const DEFAULT_GITHUB_APP_ID: &str = "4960407";
/// JWT lifetime for GitHub App authentication (must stay <= 10 minutes).
const GITHUB_APP_JWT_TTL_SECS: i64 = 540;

/// Branch carrying the rendered scaffold; the initial PR targets `main`.
pub const SCAFFOLD_BRANCH: &str = "pilot-scaffold";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotInputs {
    pub repo_name: String,
    pub runner: String,
    pub frontend_url: String,
    pub api_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvisionOutcome {
    pub repo_url: String,
    pub pr_url: String,
    pub template_version: String,
    pub files_rendered: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PilotError {
    #[error("template version must not be empty")]
    InvalidTemplateVersion,
    #[error("invalid repo name '{0}': use letters, digits, '-', '_' or '.' (max 100 chars)")]
    InvalidRepoName(String),
    #[error("pilot template dir not found (set PILOT_TEMPLATE_DIR)")]
    TemplateDirNotFound,
    #[error(
        "GitHub credentials missing: set PILOT_GITHUB_TOKEN (or GITHUB_TOKEN) or PILOT_GITHUB_APP_KEY plus PILOT_GITHUB_ORG"
    )]
    MissingCredentials,
    #[error("github app auth failed: {0}")]
    GithubAuth(String),
    #[error("create repo failed: {0}")]
    CreateRepo(String),
    #[error("render failed: {0}")]
    Render(String),
    #[error("push failed: {0}")]
    Push(String),
    #[error("open PR failed: {0}")]
    OpenPr(String),
}

/// Substitute the four `__PILOT_*` placeholders in one template file.
pub fn render_content(template: &str, inputs: &PilotInputs) -> String {
    template
        .replace(PLACEHOLDER_REPO, &inputs.repo_name)
        .replace(PLACEHOLDER_RUNNER, &inputs.runner)
        .replace(PLACEHOLDER_FRONTEND_URL, &inputs.frontend_url)
        .replace(PLACEHOLDER_API_URL, &inputs.api_url)
}

/// True when any of the four `__PILOT_*` placeholders survived rendering.
/// Only exact placeholders count: bare `__PILOT__` mentions in comments
/// (e.g. `.issue-resolver.yml`) are documentation, not render targets.
pub fn has_unrendered_placeholders(content: &str) -> bool {
    [
        PLACEHOLDER_REPO,
        PLACEHOLDER_RUNNER,
        PLACEHOLDER_FRONTEND_URL,
        PLACEHOLDER_API_URL,
    ]
    .iter()
    .any(|p| content.contains(*p))
}

/// Render raw file bytes: UTF-8 text gets placeholder substitution, other
/// files pass through untouched so binary assets stay byte-identical.
pub fn render_bytes(bytes: &[u8], inputs: &PilotInputs) -> Vec<u8> {
    match std::str::from_utf8(bytes) {
        Ok(text) => render_content(text, inputs).into_bytes(),
        Err(_) => bytes.to_vec(),
    }
}

fn validate_repo_name(name: &str) -> Result<(), PilotError> {
    let ok = !name.is_empty()
        && name.len() <= 100
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if ok {
        Ok(())
    } else {
        Err(PilotError::InvalidRepoName(name.to_owned()))
    }
}

/// Derive deterministic render inputs from the space plus env overrides.
/// Only the repo name is validated; URL values may be empty until Task 3
/// deployment targets exist (placeholders are still fully substituted).
pub fn resolve_inputs(space_id: Uuid, template_version: &str) -> Result<PilotInputs, PilotError> {
    if template_version.trim().is_empty() {
        return Err(PilotError::InvalidTemplateVersion);
    }
    let short = space_id.simple().to_string();
    let short = &short[..8];
    let repo_name = std::env::var("PILOT_REPO_NAME").unwrap_or_else(|_| format!("pilot-{short}"));
    validate_repo_name(&repo_name)?;
    Ok(PilotInputs {
        repo_name,
        runner: std::env::var("PILOT_RUNNER").unwrap_or_else(|_| "eap-backend".to_string()),
        frontend_url: std::env::var("PILOT_FRONTEND_URL").unwrap_or_default(),
        api_url: std::env::var("PILOT_API_URL").unwrap_or_default(),
    })
}

/// Locate `templates/pilot-consumer/`: explicit env first, then the
/// compile-time repo layout (dev/CI), then the prod container path.
pub fn template_dir() -> Result<PathBuf, PilotError> {
    if let Ok(dir) = std::env::var("PILOT_TEMPLATE_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return Ok(path);
        }
    }
    // CARGO_MANIFEST_DIR is backend/crates/server, so the repo-root
    // templates dir is three levels up.
    for candidate in [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../templates/pilot-consumer"),
        PathBuf::from("/app/templates/pilot-consumer"),
    ] {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }
    Err(PilotError::TemplateDirNotFound)
}

/// Collect template files as `(relative_path, bytes)`, sorted by path for
/// deterministic rendering (hidden files/dirs included).
pub fn collect_template_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, PilotError> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current).map_err(|e| PilotError::Render(e.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|e| PilotError::Render(e.to_string()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let rel = path
                    .strip_prefix(dir)
                    .map_err(|e| PilotError::Render(e.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = std::fs::read(&path).map_err(|e| PilotError::Render(e.to_string()))?;
                files.push((rel, bytes));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Render every template file with fixed inputs.
pub fn render_all(files: &[(String, Vec<u8>)], inputs: &PilotInputs) -> Vec<(String, Vec<u8>)> {
    files
        .iter()
        .map(|(rel, bytes)| (rel.clone(), render_bytes(bytes, inputs)))
        .collect()
}

struct GithubClient {
    http: reqwest::Client,
    api_base: String,
    org: String,
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GithubAppClaims {
    iss: String,
    iat: i64,
    exp: i64,
}

/// Sign a GitHub App JWT (RS256). Lifetime is capped at 10 minutes per
/// GitHub's requirement (`GITHUB_APP_JWT_TTL_SECS` = 9min + 60s clock skew).
pub fn github_app_jwt(
    app_id: &str,
    app_key_pem: &str,
    now_secs: i64,
) -> Result<String, PilotError> {
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(app_key_pem.as_bytes())
        .map_err(|e| PilotError::GithubAuth(e.to_string()))?;
    let claims = GithubAppClaims {
        iss: app_id.to_owned(),
        iat: now_secs - 60,
        exp: now_secs + GITHUB_APP_JWT_TTL_SECS,
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
        &claims,
        &key,
    )
    .map_err(|e| PilotError::GithubAuth(e.to_string()))
}

/// Static PAT from env when set and non-empty; takes priority over App auth.
fn static_token_from_env() -> Option<String> {
    for key in ["PILOT_GITHUB_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(v) = std::env::var(key) {
            if !v.trim().is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn github_api_base() -> String {
    std::env::var("GITHUB_API_URL")
        .unwrap_or_else(|_| "https://api.github.com".to_string())
        .trim_end_matches('/')
        .to_owned()
}

async fn fetch_installation_id(
    http: &reqwest::Client,
    api_base: &str,
    org: &str,
    jwt: &str,
) -> Result<u64, PilotError> {
    let res = http
        .get(format!("{api_base}/orgs/{org}/installation"))
        .bearer_auth(jwt)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| PilotError::GithubAuth(e.to_string()))?;
    if !res.status().is_success() {
        let status = res.status();
        let body: String = res
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(500)
            .collect();
        return Err(PilotError::GithubAuth(format!(
            "installation lookup: {status}: {body}"
        )));
    }
    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| PilotError::GithubAuth(e.to_string()))?;
    body.get("id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| PilotError::GithubAuth("installation lookup: missing id".to_string()))
}

async fn exchange_installation_token(
    http: &reqwest::Client,
    api_base: &str,
    installation_id: u64,
    jwt: &str,
) -> Result<String, PilotError> {
    let res = http
        .post(format!(
            "{api_base}/app/installations/{installation_id}/access_tokens"
        ))
        .bearer_auth(jwt)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| PilotError::GithubAuth(e.to_string()))?;
    if !res.status().is_success() {
        let status = res.status();
        let body: String = res
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(500)
            .collect();
        return Err(PilotError::GithubAuth(format!(
            "token exchange: {status}: {body}"
        )));
    }
    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| PilotError::GithubAuth(e.to_string()))?;
    body.get("token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_owned())
        .ok_or_else(|| PilotError::GithubAuth("token exchange: missing token".to_string()))
}

/// Normalise `PILOT_GITHUB_APP_KEY` into a PEM string. Plain PEM values pass
/// through untouched; anything else is treated as a single-line base64 encoding
/// of the PEM. The base64 form exists because systemd's `EnvironmentFile` only
/// reads the first line of a value, which would truncate a multi-line PEM to its
/// BEGIN header (`InvalidKeyFormat`); base64 is also whitespace-free, so it
/// survives the env file unquoted.
pub fn normalize_app_key(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    // `-----` cannot appear in base64, so this unambiguously detects raw PEM.
    if raw.contains("-----BEGIN") {
        return Some(raw.to_owned());
    }
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(raw.as_bytes())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|pem| pem.contains("-----BEGIN"))
}

/// Read GitHub App credentials from env (`PILOT_GITHUB_APP_ID` defaults to
/// `DEFAULT_GITHUB_APP_ID`; `PILOT_GITHUB_APP_KEY` PEM is required).
fn github_app_credentials() -> Result<(String, String), PilotError> {
    let app_id =
        std::env::var("PILOT_GITHUB_APP_ID").unwrap_or_else(|_| DEFAULT_GITHUB_APP_ID.to_string());
    if app_id.trim().is_empty() {
        return Err(PilotError::MissingCredentials);
    }
    let pem = std::env::var("PILOT_GITHUB_APP_KEY")
        .ok()
        .and_then(|raw| normalize_app_key(&raw))
        .ok_or(PilotError::MissingCredentials)?;
    Ok((app_id.trim().to_owned(), pem))
}

/// Mint an installation token for `PILOT_GITHUB_ORG` via the GitHub App.
async fn github_app_token(
    http: &reqwest::Client,
    api_base: &str,
    org: &str,
) -> Result<String, PilotError> {
    let (app_id, pem) = github_app_credentials()?;
    let now = chrono::Utc::now().timestamp();
    let jwt = github_app_jwt(&app_id, &pem, now)?;
    let installation_id = fetch_installation_id(http, api_base, org, &jwt).await?;
    exchange_installation_token(http, api_base, installation_id, &jwt).await
}

/// Resolve the bearer token: static PAT first (backward compatible),
/// otherwise mint one from the GitHub App installation.
async fn resolve_github_token(
    http: &reqwest::Client,
    api_base: &str,
    org: &str,
) -> Result<String, PilotError> {
    if let Some(token) = static_token_from_env() {
        return Ok(token);
    }
    github_app_token(http, api_base, org).await
}

impl GithubClient {
    async fn from_env() -> Result<Self, PilotError> {
        let org = std::env::var("PILOT_GITHUB_ORG").map_err(|_| PilotError::MissingCredentials)?;
        if org.trim().is_empty() {
            return Err(PilotError::MissingCredentials);
        }
        let api_base = github_api_base();
        let http = reqwest::Client::builder()
            .user_agent("eap-pilot-provisioner")
            .build()
            .map_err(|e| PilotError::CreateRepo(e.to_string()))?;
        let token = resolve_github_token(&http, &api_base, org.trim()).await?;
        Ok(Self {
            http,
            api_base,
            org: org.trim().to_owned(),
            token,
        })
    }

    /// Create the pilot repo and return `(html_url, default_branch)`.
    /// The caller threads `default_branch` through the push + PR steps so a
    /// renamed org default (not `main`) still yields common history.
    async fn create_repo(&self, name: &str) -> Result<(String, String), PilotError> {
        let res = self
            .http
            .post(format!("{}/orgs/{}/repos", self.api_base, self.org))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "name": name,
                "private": true,
                "auto_init": true,
            }))
            .send()
            .await
            .map_err(|e| PilotError::CreateRepo(e.to_string()))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            let body: String = body.chars().take(500).collect();
            return Err(PilotError::CreateRepo(format!("{status}: {body}")));
        }
        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| PilotError::CreateRepo(e.to_string()))?;
        // The repo already exists remotely at this point, but provision only
        // enters with_rollback (which owns delete_repo) afterwards — so a
        // parse failure here must clean up the orphan itself.
        let parsed = (|| {
            let html_url = body
                .get("html_url")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_owned())
                .ok_or_else(|| "missing html_url".to_string())?;
            let default_branch = body
                .get("default_branch")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_owned())
                .ok_or_else(|| "missing default_branch".to_string())?;
            Ok::<(String, String), String>((html_url, default_branch))
        })();
        match parsed {
            Ok(v) => Ok(v),
            Err(e) => {
                self.delete_repo(name).await;
                Err(PilotError::CreateRepo(e))
            }
        }
    }

    /// Best-effort rollback: deleting the repo must never mask the original error.
    async fn delete_repo(&self, name: &str) {
        let res = self
            .http
            .delete(format!("{}/repos/{}/{}", self.api_base, self.org, name))
            .bearer_auth(&self.token)
            .send()
            .await;
        match res {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => {
                tracing::warn!(repo = name, status = %r.status(), "pilot rollback delete failed")
            }
            Err(e) => tracing::warn!(repo = name, error = %e, "pilot rollback delete failed"),
        }
    }

    async fn open_pr(
        &self,
        repo: &str,
        template_version: &str,
        base_branch: &str,
    ) -> Result<String, PilotError> {
        let res = self
            .http
            .post(format!(
                "{}/repos/{}/{}/pulls",
                self.api_base, self.org, repo
            ))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "title": format!("chore: pilot scaffold (template {template_version})"),
                "head": SCAFFOLD_BRANCH,
                "base": base_branch,
                "body": "Pilot generator initial scaffold. Thin-shell: workflows pin L1, no business code.",
            }))
            .send()
            .await
            .map_err(|e| PilotError::OpenPr(e.to_string()))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            let body: String = body.chars().take(500).collect();
            return Err(PilotError::OpenPr(format!("{status}: {body}")));
        }
        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| PilotError::OpenPr(e.to_string()))?;
        body.get("html_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
            .ok_or_else(|| PilotError::OpenPr("missing html_url".to_string()))
    }
}

/// Push rendered files as `SCAFFOLD_BRANCH` through the GitHub Git Data API
/// (`api.github.com` only — never touches `github.com`'s git port, which is
/// intermittently blocked from our network). The commit is parented on the
/// base tip so the branch shares history with it (the old CLI push cut the
/// branch from the fetched tip for the same reason: no `422 no history in
/// common` on open_pr).
///
/// Text files ride inline in the tree payload; non-UTF8 files go through a
/// blob first (the tree API only accepts UTF-8 `content`). All scaffold files
/// land as mode 100644 — the template has no executables.
impl GithubClient {
    fn api_push_err(&self, ctx: &str, status: reqwest::StatusCode, body: &str) -> PilotError {
        let body: String = body.chars().take(500).collect();
        PilotError::Push(scrub_push_output(
            &format!("{ctx} failed: {status}: {body}"),
            &self.token,
            "<redacted>",
        ))
    }

    async fn api_get_json(&self, path: &str, ctx: &str) -> Result<serde_json::Value, PilotError> {
        let res = self
            .http
            .get(format!("{}{path}", self.api_base))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| PilotError::Push(format!("{ctx} failed: {e}")))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(self.api_push_err(ctx, status, &body));
        }
        res.json()
            .await
            .map_err(|e| PilotError::Push(format!("{ctx} failed: {e}")))
    }

    async fn api_post_json(
        &self,
        path: &str,
        ctx: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, PilotError> {
        self.api_send_json(reqwest::Method::POST, path, ctx, payload)
            .await
    }

    async fn api_patch_json(
        &self,
        path: &str,
        ctx: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, PilotError> {
        self.api_send_json(reqwest::Method::PATCH, path, ctx, payload)
            .await
    }

    async fn api_send_json(
        &self,
        method: reqwest::Method,
        path: &str,
        ctx: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, PilotError> {
        let res = self
            .http
            .request(method, format!("{}{path}", self.api_base))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(payload)
            .send()
            .await
            .map_err(|e| PilotError::Push(format!("{ctx} failed: {e}")))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(self.api_push_err(ctx, status, &body));
        }
        res.json()
            .await
            .map_err(|e| PilotError::Push(format!("{ctx} failed: {e}")))
    }

    fn response_sha(
        body: &serde_json::Value,
        ctx: &str,
        field: &str,
    ) -> Result<String, PilotError> {
        body.get(field)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .ok_or_else(|| PilotError::Push(format!("{ctx} failed: missing {field}")))
    }

    /// Resolve the `heads/<base>` tip SHA. `create_repo` uses `auto_init`, so
    /// the base always exists with an initial commit.
    async fn base_tip_sha(&self, repo: &str, base_branch: &str) -> Result<String, PilotError> {
        // Base arrives from the create-repo response; validate before
        // interpolating into the URL path.
        validate_base_branch(base_branch)?;
        let body = self
            .api_get_json(
                &format!("/repos/{}/{}/git/ref/heads/{}", self.org, repo, base_branch),
                "resolve base tip",
            )
            .await?;
        body.get("object")
            .and_then(|o| o.get("sha"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .ok_or_else(|| {
                PilotError::Push("resolve base tip failed: missing object.sha".to_string())
            })
    }

    /// Resolve the tree SHA of a commit: the tree API's `base_tree` must be
    /// a tree object, while `base_tip_sha` returns the commit the ref points
    /// to — passing the commit straight through risks a 422.
    async fn commit_tree_sha(&self, repo: &str, commit_sha: &str) -> Result<String, PilotError> {
        let body = self
            .api_get_json(
                &format!("/repos/{}/{}/git/commits/{}", self.org, repo, commit_sha),
                "resolve commit tree",
            )
            .await?;
        body.get("tree")
            .and_then(|t| t.get("sha"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .ok_or_else(|| {
                PilotError::Push("resolve commit tree failed: missing tree.sha".to_string())
            })
    }

    async fn create_blob(&self, repo: &str, bytes: &[u8]) -> Result<String, PilotError> {
        use base64::Engine as _;
        let body = self
            .api_post_json(
                &format!("/repos/{}/{}/git/blobs", self.org, repo),
                "create blob",
                &serde_json::json!({
                    "content": base64::engine::general_purpose::STANDARD.encode(bytes),
                    "encoding": "base64",
                }),
            )
            .await?;
        Self::response_sha(&body, "create blob", "sha")
    }

    async fn create_scaffold_tree(
        &self,
        repo: &str,
        base_sha: &str,
        rendered: &[(String, Vec<u8>)],
    ) -> Result<String, PilotError> {
        let mut tree = Vec::with_capacity(rendered.len());
        for (rel, bytes) in rendered {
            let entry = match std::str::from_utf8(bytes) {
                Ok(text) => serde_json::json!({
                    "path": rel,
                    "mode": "100644",
                    "type": "blob",
                    "content": text,
                }),
                Err(_) => {
                    let sha = self.create_blob(repo, bytes).await?;
                    serde_json::json!({
                        "path": rel,
                        "mode": "100644",
                        "type": "blob",
                        "sha": sha,
                    })
                }
            };
            tree.push(entry);
        }
        let body = self
            .api_post_json(
                &format!("/repos/{}/{}/git/trees", self.org, repo),
                "create tree",
                &serde_json::json!({ "base_tree": base_sha, "tree": tree }),
            )
            .await?;
        Self::response_sha(&body, "create tree", "sha")
    }

    async fn create_scaffold_commit(
        &self,
        repo: &str,
        tree_sha: &str,
        parent_sha: &str,
    ) -> Result<String, PilotError> {
        let body = self
            .api_post_json(
                &format!("/repos/{}/{}/git/commits", self.org, repo),
                "create commit",
                &serde_json::json!({
                    "message": "chore: pilot scaffold",
                    "tree": tree_sha,
                    "parents": [parent_sha],
                }),
            )
            .await?;
        Self::response_sha(&body, "create commit", "sha")
    }

    /// Point `pilot-scaffold` at the new commit. A retry after a partial
    /// success (ref created, later step failed, rollback delete missed) hits
    /// `422 Reference already exists` on POST — fall back to a non-force
    /// PATCH so a genuinely diverged branch still errors instead of being
    /// silently overwritten.
    async fn create_branch_ref(&self, repo: &str, commit_sha: &str) -> Result<(), PilotError> {
        let res = self
            .http
            .post(format!(
                "{}/repos/{}/{}/git/refs",
                self.api_base, self.org, repo
            ))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&serde_json::json!({
                "ref": format!("refs/heads/{SCAFFOLD_BRANCH}"),
                "sha": commit_sha,
            }))
            .send()
            .await
            .map_err(|e| PilotError::Push(format!("create branch ref failed: {e}")))?;
        if res.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
            let body = res.text().await.unwrap_or_default();
            if body.contains("already exists") {
                let patched = self
                    .api_patch_json(
                        &format!(
                            "/repos/{}/{}/git/refs/heads/{SCAFFOLD_BRANCH}",
                            self.org, repo
                        ),
                        "update branch ref",
                        &serde_json::json!({ "sha": commit_sha, "force": false }),
                    )
                    .await?;
                return patched
                    .get("ref")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|_| ())
                    .ok_or_else(|| {
                        PilotError::Push("update branch ref failed: missing ref".to_string())
                    });
            }
            return Err(self.api_push_err(
                "create branch ref",
                reqwest::StatusCode::UNPROCESSABLE_ENTITY,
                &body,
            ));
        }
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(self.api_push_err("create branch ref", status, &body));
        }
        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| PilotError::Push(format!("create branch ref failed: {e}")))?;
        body.get("ref")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|_| ())
            .ok_or_else(|| PilotError::Push("create branch ref failed: missing ref".to_string()))
    }

    async fn push_rendered_via_api(
        &self,
        repo: &str,
        rendered: &[(String, Vec<u8>)],
        base_branch: &str,
    ) -> Result<(), PilotError> {
        let tip = self.base_tip_sha(repo, base_branch).await?;
        let base_tree = self.commit_tree_sha(repo, &tip).await?;
        let tree = self
            .create_scaffold_tree(repo, &base_tree, rendered)
            .await?;
        let commit = self.create_scaffold_commit(repo, &tree, &tip).await?;
        self.create_branch_ref(repo, &commit).await
    }
}
/// Scrub push secrets from surfaced text. Only the credential material is
/// replaced — host/org/repo stay visible for diagnostics. Covers three
/// forms: the `x-access-token:<token>@` URL fragment, the bare token
/// (truncated/wrapped echoes), and its percent-encoded form.
fn scrub_push_output(s: &str, token: &str, redact: &str) -> String {
    if token.is_empty() {
        return s.to_owned();
    }
    let cred = format!("x-access-token:{token}@");
    let s = s.replace(&cred, &format!("x-access-token:{redact}@"));
    let s = s.replace(token, redact);
    let encoded: String =
        percent_encoding::utf8_percent_encode(token, percent_encoding::NON_ALPHANUMERIC)
            .to_string();
    if encoded == token {
        s
    } else {
        s.replace(&encoded, redact)
    }
}

/// Base branch names arrive from the GitHub create-repo response; validate
/// before interpolating into git argv so a hostile/empty value can neither
/// be parsed as an option nor break ref resolution.
fn validate_base_branch(name: &str) -> Result<(), PilotError> {
    let ok = !name.is_empty()
        && name.len() <= 255
        && !name.starts_with(['-', '/', '.'])
        && !name.contains("..")
        && !name.contains("@{")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err(PilotError::Push(format!("invalid base branch: {name}")))
    }
}

/// Minimal closed loop: create repo -> render -> push -> open PR.
/// Steps 2-4 failing trigger rollback deletion of the created repo.
pub async fn provision_pilot_consumer(
    space_id: Uuid,
    template_version: &str,
) -> Result<ProvisionOutcome, PilotError> {
    let inputs = resolve_inputs(space_id, template_version)?;
    let template_version = template_version.trim().to_owned();
    let github = GithubClient::from_env().await?;

    let (repo_url, base_branch) = github.create_repo(&inputs.repo_name).await?;

    let with_rollback = async {
        let dir = template_dir()?;
        let files = collect_template_files(&dir)?;
        let rendered = render_all(&files, &inputs);
        for (rel, bytes) in &rendered {
            if let Ok(text) = std::str::from_utf8(bytes) {
                if has_unrendered_placeholders(text) {
                    return Err::<(String, usize), PilotError>(PilotError::Render(format!(
                        "unrendered placeholder left in {rel}"
                    )));
                }
            }
        }
        github
            .push_rendered_via_api(&inputs.repo_name, &rendered, &base_branch)
            .await?;
        let pr_url = github
            .open_pr(&inputs.repo_name, &template_version, &base_branch)
            .await?;
        Ok::<(String, usize), PilotError>((pr_url, rendered.len()))
    }
    .await;

    match with_rollback {
        Ok((pr_url, files_rendered)) => Ok(ProvisionOutcome {
            repo_url,
            pr_url,
            template_version,
            files_rendered,
        }),
        Err(e) => {
            github.delete_repo(&inputs.repo_name).await;
            Err(e)
        }
    }
}

/// Register `provisionPilotConsumer(spaceId, templateVersion)` without
/// touching the existing `ensure_*` gate logic: space ACL is enforced here
/// through `SpaceService` directly with the same edit rule.
pub fn register_pilot_mutations(builder: &mut seaography::Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, InputValue, TypeRef};

    let field = Field::new(
        "provisionPilotConsumer",
        TypeRef::named_nn(TypeRef::STRING),
        |ctx| {
            FieldFuture::new(async move {
                let claims = ctx.data_opt::<crate::middleware::Claims>().ok_or_else(|| {
                    async_graphql::Error::new("Authentication required for mutations.")
                })?;
                if !claims.user_role().can_create() {
                    return Err(async_graphql::Error::new(
                        "Insufficient permissions for this operation.",
                    ));
                }
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id_str = ctx.args.try_get("spaceId")?.string()?;
                let space_id = Uuid::parse_str(space_id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;
                let template_version = ctx.args.try_get("templateVersion")?.string()?.to_owned();

                let service = SpaceService::new(
                    SeaOrmSpaceRepo::new(db.clone()),
                    SeaOrmMembershipRepo::new(db.clone()),
                    SeaOrmAuditLogRepo::new(db.clone()),
                )
                .with_strict_audit();
                service
                    .ensure_can_edit(space_id, claims.user_id, claims.user_role())
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                let outcome = provision_pilot_consumer(space_id, &template_version)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let json = serde_json::to_string(&outcome)
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::String(json)))
            })
        },
    )
    .argument(InputValue::new(
        "spaceId",
        TypeRef::named_nn(TypeRef::STRING),
    ))
    .argument(InputValue::new(
        "templateVersion",
        TypeRef::named_nn(TypeRef::STRING),
    ));

    builder.mutations.push(field);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_inputs() -> PilotInputs {
        PilotInputs {
            repo_name: "pilot-demo".to_string(),
            runner: "eap-backend".to_string(),
            frontend_url: "https://pilot.xieyucheng.top".to_string(),
            api_url: "https://pilot-api.xieyucheng.top".to_string(),
        }
    }

    #[test]
    fn pilot_render_snapshot() {
        let inputs = fixture_inputs();
        let template = "ghcr-image: ghcr.io/link-seek/__PILOT_REPO__\n\
             runner-label: __PILOT_RUNNER__\n\
             frontend-url: __PILOT_FRONTEND_URL__\n\
             api-url: __PILOT_API_URL__\n";
        let expected = "ghcr-image: ghcr.io/link-seek/pilot-demo\n\
             runner-label: eap-backend\n\
             frontend-url: https://pilot.xieyucheng.top\n\
             api-url: https://pilot-api.xieyucheng.top\n";
        let rendered = render_content(template, &inputs);
        assert_eq!(rendered, expected);
        assert!(!has_unrendered_placeholders(&rendered));
    }

    #[test]
    fn pilot_render_replaces_all_placeholders() {
        let inputs = fixture_inputs();
        let template = [
            PLACEHOLDER_REPO,
            PLACEHOLDER_RUNNER,
            PLACEHOLDER_FRONTEND_URL,
            PLACEHOLDER_API_URL,
        ]
        .join("|");
        let rendered = render_content(&template, &inputs);
        assert_eq!(
            rendered,
            "pilot-demo|eap-backend|https://pilot.xieyucheng.top|https://pilot-api.xieyucheng.top"
        );
        assert!(!has_unrendered_placeholders(&rendered));
    }

    #[test]
    fn pilot_render_resolve_inputs_deterministic() {
        let space_id = Uuid::parse_str("11111111-2222-4333-8444-555555555555").unwrap();
        let first = resolve_inputs(space_id, "v1.0.0-pilot").unwrap();
        let second = resolve_inputs(space_id, "v1.0.0-pilot").unwrap();
        assert_eq!(first, second);
        if std::env::var("PILOT_REPO_NAME").is_err() {
            assert_eq!(first.repo_name, "pilot-11111111");
        }
        assert!(resolve_inputs(space_id, "  ").is_err());
        assert!(resolve_inputs(space_id, "").is_err());
    }

    #[test]
    fn pilot_render_rejects_unsafe_repo_name() {
        for bad in ["", "../evil", "a/b", "x y", &"a".repeat(101)] {
            assert!(
                matches!(validate_repo_name(bad), Err(PilotError::InvalidRepoName(_))),
                "must reject {bad:?}"
            );
        }
        assert!(validate_repo_name("pilot-demo").is_ok());
    }

    #[test]
    fn pilot_render_ignores_bare_marker_in_comments() {
        assert!(!has_unrendered_placeholders(
            "# 由生成器渲染时替换 __PILOT__ 占位"
        ));
        assert!(has_unrendered_placeholders("image: __PILOT_REPO__"));
    }

    #[test]
    fn pilot_render_real_template_tree() {
        let dir = match template_dir() {
            Ok(d) => d,
            Err(_) => return,
        };
        let inputs = fixture_inputs();
        let files = collect_template_files(&dir).expect("collect template files");
        assert!(!files.is_empty());
        assert!(files.iter().any(|(r, _)| r.ends_with("on-push.yml")));
        let rendered = render_all(&files, &inputs);
        assert_eq!(rendered.len(), files.len());
        for (rel, bytes) in &rendered {
            if let Ok(text) = std::str::from_utf8(bytes) {
                assert!(
                    !has_unrendered_placeholders(text),
                    "leftover placeholder in {rel}"
                );
            }
        }
        let on_push = rendered
            .iter()
            .find(|(r, _)| r.ends_with("on-push.yml"))
            .expect("on-push.yml present");
        let text = String::from_utf8(on_push.1.clone()).unwrap();
        assert!(text.contains("ghcr.io/link-seek/pilot-demo"));
        assert!(
            !rendered
                .iter()
                .any(|(_, b)| { String::from_utf8_lossy(b).contains("__PILOT_OSS_BUCKET__") }),
            "OSS placeholder must be gone after render"
        );
    }

    #[test]
    fn pilot_render_bytes_passthrough_binary() {
        let inputs = fixture_inputs();
        let binary = vec![0xff, 0xfe, 0x00, 0x01];
        assert_eq!(render_bytes(&binary, &inputs), binary);
    }

    const TEST_APP_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
        MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDtNV1hRBX5J/HF\n\
        3YTy3PxJyzOHAug4HRplME67yoeIh3waNW78us2ZzGd7NAhhrhqpPGFslYj/qvEm\n\
        AH9XVseALp/Jo41OtHtRm5bbLQbZizzA1WU+3VctA9v2APbyTVR/RsQgDAMLxVmp\n\
        AMXvOlI9RVzIBjjcMjiGLn/S8TtRbSxI8n/KIS+M8WsVw9n/1oZK1AL12pYDe4Ln\n\
        HitU+4rqGf7rwD1ORXuPM+kq1BBHIWWeZ5qjBiONoVUplqR5FJgPEevyZxdCUpx1\n\
        dPDVNXQVNniXhWFzuCt9n+aHlfwiVgNt/soe3bo/ZhRDazNwy6eINX8/mNi/atyU\n\
        ISITpWi9AgMBAAECggEAC7ccwUrITgmpwHS4FfmRkUByr0qWtvzC+rfnz5EJYBYW\n\
        7EFy1ZMRR/UHMFfJyS883Fph0mfRQBVMeyy/nUvpJvzGggIsnrQ9ufJWAUWoRrLA\n\
        gaKYcUIjxdKguLXj/GQS1gVj9tQ5C0oIK1dhLzdBbArCshNSmBd34LKnt/6XiCYk\n\
        mAwj6rlxkNlzzLhM5mDf8lmeOcnkrJnMUYFj2Lsl6yXqG8RQLYjY5TK8vZN7zJc3\n\
        rpUWCDa+W27srPWYTYmy3gV4tkmUIXIFll8wYYU5y6k0gfi/5yOFm9PlWHSSWBD7\n\
        cl8gYPbfPrqe/0W240LT5oy9ragOOWq3z+GdiFVBiQKBgQD30K+s39tPajg47bOl\n\
        nVfT4xk5hRHtZQ548OzqCD838RFcH7J63jbP16ZbudCGVItHJ0jH57UqIRKdyFSQ\n\
        wxi8VMFjziWu2Kz8bpquxbMjUB7yW+65O+B1AYTWBSvQ0vV3AiHydwcCVgxLlBNq\n\
        MRnZ7MX/sA/GGfv6PTJqDZXqBQKBgQD1Cv9DPGuyrJ5K/1fK+x7As/gJkk8tQY4B\n\
        u18KhOkyYiPXyz/PJNvhMvwblX81ZIlS+m5SLxrAWG0X/+UGUGkDKEjXaRoBxQFt\n\
        nT5J/iPV1LFTFgT12v60igaGyHT7Ju98ZaXu15IM3iZqb5QKlJopZxruZWF/M6UB\n\
        GS/57k9pWQKBgCwXCOps+Yvrjg0y3V992v5rzTUao9HkxOpnkv8gcH73eOs3CH4r\n\
        wvy/lW2EZcFAkXcbWiuW4fiY4cMIvWL0ExaOzcmAB9xP2Jcg5oxpyDFkM91S1epG\n\
        6OxoVMXvLZh9sAZ4bqnA25Ji1NUthzbBfaP0KFYRcP0B6n7fHHUZ7a4xAoGABv3v\n\
        Vq3MrOZ8BcvPZ31O3VTFSRChrbrnIGmGRriQJt3iA/BKu9BjbcOUqfzUCmP5/yIi\n\
        L7okW0SqqDqnAE0fEfX+ThczpMVISyZndpkH0Lwm6yX/sjwzdFdT5Fin7dqojrYf\n\
        y/betftIwVS5tquS0oecnxzJcWW52ZQsaEdCgNECgYEAm7MC2sfjSCJtEj2809AT\n\
        6CYSlafOKLirm9ap0iwpzBbXtTu0svG0dXwGvlQe3cP2i6yIXI7oxNeWTz9H8sdf\n\
        JkC716D+odIast8+D1fg/IklPzI4n81T8nSl1lZYzMOMfDolBJc0Y97Jq2wKj9SW\n\
        EG47HEHWgcBSHGH2d+EI2J0=\n\
        -----END PRIVATE KEY-----\n";

    const TEST_APP_PUB_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
        MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA7TVdYUQV+Sfxxd2E8tz8\n\
        ScszhwLoOB0aZTBOu8qHiId8GjVu/LrNmcxnezQIYa4aqTxhbJWI/6rxJgB/V1bH\n\
        gC6fyaONTrR7UZuW2y0G2Ys8wNVlPt1XLQPb9gD28k1Uf0bEIAwDC8VZqQDF7zpS\n\
        PUVcyAY43DI4hi5/0vE7UW0sSPJ/yiEvjPFrFcPZ/9aGStQC9dqWA3uC5x4rVPuK\n\
        6hn+68A9TkV7jzPpKtQQRyFlnmeaowYjjaFVKZakeRSYDxHr8mcXQlKcdXTw1TV0\n\
        FTZ4l4Vhc7grfZ/mh5X8IlYDbf7KHt26P2YUQ2szcMuniDV/P5jYv2rclCEiE6Vo\n\
        vQIDAQAB\n\
        -----END PUBLIC KEY-----\n";

    static ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
        let _guard = ENV_GUARD.lock().unwrap();
        let mut saved = Vec::new();
        for (k, v) in vars {
            saved.push(((*k).to_string(), std::env::var(k).ok()));
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        f();
        for (k, v) in saved {
            match v {
                Some(val) => std::env::set_var(&k, val),
                None => std::env::remove_var(&k),
            }
        }
    }

    #[test]
    fn pilot_github_app_jwt_rs256() {
        let now = chrono::Utc::now().timestamp();
        let jwt = github_app_jwt(DEFAULT_GITHUB_APP_ID, TEST_APP_KEY_PEM, now).unwrap();
        assert_eq!(jwt.split('.').count(), 3);
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&[DEFAULT_GITHUB_APP_ID]);
        validation.validate_exp = false;
        let key = jsonwebtoken::DecodingKey::from_rsa_pem(TEST_APP_PUB_PEM.as_bytes()).unwrap();
        let data = jsonwebtoken::decode::<GithubAppClaims>(&jwt, &key, &validation).unwrap();
        assert_eq!(data.claims.iss, DEFAULT_GITHUB_APP_ID);
        assert_eq!(
            data.claims.exp - data.claims.iat,
            GITHUB_APP_JWT_TTL_SECS + 60
        );
        assert!(data.claims.exp - now <= 600);
        assert!(data.claims.exp - now > 0);
        assert!(github_app_jwt("4960407", "not-a-pem", now).is_err());
    }

    #[test]
    fn pilot_app_key_accepts_pem_and_base64() {
        use base64::Engine as _;
        let pem = TEST_APP_KEY_PEM.trim();
        // Raw multi-line PEM passes through untouched.
        assert_eq!(normalize_app_key(TEST_APP_KEY_PEM).as_deref(), Some(pem));
        // Single-line base64 (what deploy.sh writes into EnvironmentFile).
        let b64 = base64::engine::general_purpose::STANDARD.encode(pem);
        assert!(!b64.contains('\n'));
        assert_eq!(normalize_app_key(&b64).as_deref(), Some(pem));
        // The decoded key must still sign a valid App JWT.
        let decoded = normalize_app_key(&b64).unwrap();
        let now = chrono::Utc::now().timestamp();
        assert!(github_app_jwt(DEFAULT_GITHUB_APP_ID, &decoded, now).is_ok());
        // Rejects empty, invalid base64 and base64 payloads that are not PEM.
        assert!(normalize_app_key("   ").is_none());
        assert!(normalize_app_key("not base64 !!").is_none());
        let not_pem = base64::engine::general_purpose::STANDARD.encode("hello world");
        assert!(normalize_app_key(&not_pem).is_none());
    }

    #[test]
    fn pilot_static_token_takes_priority() {
        with_env(
            &[
                ("PILOT_GITHUB_TOKEN", Some("static-pat")),
                ("GITHUB_TOKEN", Some("fallback-pat")),
                ("PILOT_GITHUB_APP_KEY", None),
            ],
            || {
                assert_eq!(static_token_from_env().as_deref(), Some("static-pat"));
            },
        );
        with_env(
            &[
                ("PILOT_GITHUB_TOKEN", None),
                ("GITHUB_TOKEN", Some("fallback-pat")),
            ],
            || {
                assert_eq!(static_token_from_env().as_deref(), Some("fallback-pat"));
            },
        );
        with_env(
            &[("PILOT_GITHUB_TOKEN", Some("  ")), ("GITHUB_TOKEN", None)],
            || {
                assert!(static_token_from_env().is_none());
            },
        );
    }

    #[test]
    fn pilot_app_auth_missing_key_fails_fast() {
        with_env(
            &[
                ("PILOT_GITHUB_TOKEN", None),
                ("GITHUB_TOKEN", None),
                ("PILOT_GITHUB_APP_KEY", None),
            ],
            || {
                assert!(matches!(
                    github_app_credentials(),
                    Err(PilotError::MissingCredentials)
                ));
            },
        );
        with_env(
            &[
                ("PILOT_GITHUB_APP_ID", None),
                (
                    "PILOT_GITHUB_APP_KEY",
                    Some("-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n"),
                ),
            ],
            || {
                let (app_id, _) = github_app_credentials().unwrap();
                assert_eq!(app_id, DEFAULT_GITHUB_APP_ID);
            },
        );
    }

    #[tokio::test]
    async fn pilot_installation_token_exchange_mock() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap();
                let req = String::from_utf8_lossy(&buf[..n]).into_owned();
                let body =
                    if req.starts_with("GET ") && req.contains("/orgs/link-seek/installation") {
                        "{\"id\": 162075552}".to_string()
                    } else if req.starts_with("POST ")
                        && req.contains("/app/installations/162075552/access_tokens")
                    {
                        assert!(req.contains("Bearer "));
                        "{\"token\": \"ghs_mock_installation_token\"}".to_string()
                    } else {
                        panic!(
                            "unexpected mock request: {}",
                            req.lines().next().unwrap_or("")
                        );
                    };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(resp.as_bytes()).await.unwrap();
            }
        });

        let http = reqwest::Client::builder().build().unwrap();
        let api_base = format!("http://{addr}");
        let jwt = github_app_jwt(DEFAULT_GITHUB_APP_ID, TEST_APP_KEY_PEM, 1_786_000_000).unwrap();
        let id = fetch_installation_id(&http, &api_base, "link-seek", &jwt)
            .await
            .unwrap();
        assert_eq!(id, 162075552);
        let token = exchange_installation_token(&http, &api_base, id, &jwt)
            .await
            .unwrap();
        assert_eq!(token, "ghs_mock_installation_token");
        server.await.unwrap();
    }

    #[test]
    fn pilot_scrub_redacts_credential_forms() {
        // Token with a char that percent-encoding transforms ('_').
        let token = "ghi_安装令牌_xyz";
        let url = format!("https://x-access-token:{token}@github.com/link-seek/pilot-consumer-gen.git");
        let text = format!(
            "fatal: unable to access '{url}': git said '{token}' then '{enc}'",
            enc = percent_encoding::utf8_percent_encode(
                token,
                percent_encoding::NON_ALPHANUMERIC
            )
        );
        let scrubbed = scrub_push_output(&text, token, "<redacted>");
        assert!(!scrubbed.contains(token), "bare token leaked: {scrubbed}");
        assert!(
            scrubbed.contains("x-access-token:<redacted>@"),
            "credential not redacted in place: {scrubbed}"
        );
        // Diagnostics must survive: host/org/repo stay visible.
        for keep in ["github.com", "link-seek", "pilot-consumer-gen"] {
            assert!(scrubbed.contains(keep), "lost diagnostics: {scrubbed}");
        }
        // Empty token must not nuke the whole string.
        assert_eq!(scrub_push_output("abc", "", "<redacted>"), "abc");
        // Bad branch names are rejected before reaching git argv.
        assert!(validate_base_branch("main").is_ok());
        assert!(validate_base_branch("").is_err());
        assert!(validate_base_branch("-f").is_err());
        assert!(validate_base_branch("a..b").is_err());
    }

    /// Read one full HTTP request: headers first, then exactly
    /// Content-Length body bytes (a single `read` may return fragments).
    async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            let n = stream.read(&mut tmp).await.unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&buf[..end]).into_owned();
                let len = headers
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("Content-Length:")
                            .or_else(|| l.strip_prefix("content-length:"))
                    })
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if buf.len() >= end + 4 + len {
                    break;
                }
            }
            if buf.len() > 1_048_576 {
                break;
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Regression test for the Data-API push: the scaffold tree overlays the
    /// base commit's tree, the commit is parented on the base tip (shares
    /// history, no 422 on open_pr), text files ride inline in the tree,
    /// non-UTF8 files go through a blob, and the branch ref points at the
    /// new commit. Local mock server, no network, no git binary. Uses a
    /// non-`main` base (`trunk`) to prove nothing hardcodes `main`.
    #[tokio::test]
    async fn pilot_data_api_push_order_and_payloads() {
        use base64::Engine as _;
        use std::sync::{Arc, Mutex};
        use tokio::io::AsyncWriteExt;

        let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let seen_srv = seen.clone();
        let server = tokio::spawn(async move {
            // Exactly the 6 calls push_rendered_via_api makes for one
            // binary file: ref, commit-tree, blob, tree, commit, ref-create.
            for _ in 0..6 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let req = read_request(&mut stream).await;
                let head = req.lines().next().unwrap_or("").to_owned();
                let body = req.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
                seen_srv.lock().unwrap().push(format!("{head} || {body}"));
                let payload = if head.starts_with("GET ") && head.contains("/git/ref/heads/") {
                    assert!(
                        head.contains("/repos/o/r/git/ref/heads/trunk"),
                        "base ref path: {head}"
                    );
                    "{\"object\":{\"sha\":\"TIP\"}}".to_string()
                } else if head.starts_with("GET ") && head.contains("/git/commits/") {
                    assert!(
                        head.contains("/repos/o/r/git/commits/TIP"),
                        "tree of tip: {head}"
                    );
                    "{\"tree\":{\"sha\":\"BASETREE\"}}".to_string()
                } else if head.starts_with("POST ") && head.contains("/git/blobs") {
                    "{\"sha\":\"BLOB1\"}".to_string()
                } else if head.starts_with("POST ") && head.contains("/git/trees") {
                    "{\"sha\":\"TREE1\"}".to_string()
                } else if head.starts_with("POST ") && head.contains("/git/commits") {
                    "{\"sha\":\"COMMIT1\"}".to_string()
                } else if head.starts_with("POST ") && head.contains("/git/refs") {
                    "{\"ref\":\"refs/heads/pilot-scaffold\",\"object\":{\"sha\":\"COMMIT1\"}}"
                        .to_string()
                } else {
                    panic!("unexpected mock request: {head}");
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    payload.len(),
                    payload
                );
                stream.write_all(resp.as_bytes()).await.unwrap();
            }
        });

        let http = reqwest::Client::builder().build().unwrap();
        let github = GithubClient {
            http,
            api_base: format!("http://{addr}"),
            org: "o".to_string(),
            token: "DUMMY".to_string(),
        };
        let rendered = vec![
            ("a.txt".to_string(), b"hello".to_vec()),
            ("bin.dat".to_string(), vec![0xff, 0xfe, 0x00, 0x01]),
        ];
        github
            .push_rendered_via_api("r", &rendered, "trunk")
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), server)
            .await
            .expect("mock server hung")
            .unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(
            seen.len(),
            6,
            "expected ref+tree-of-tip+blob+tree+commit+ref, got {seen:?}"
        );

        let blob_body = seen
            .iter()
            .find(|s| s.contains("POST ") && s.contains("/git/blobs"))
            .expect("blob call");
        let blob: serde_json::Value =
            serde_json::from_str(blob_body.split(" || ").nth(1).unwrap()).unwrap();
        assert_eq!(blob["encoding"], "base64");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(blob["content"].as_str().unwrap())
            .unwrap();
        assert_eq!(decoded, vec![0xff, 0xfe, 0x00, 0x01]);

        let tree_body = seen
            .iter()
            .find(|s| s.contains("POST ") && s.contains("/git/trees"))
            .expect("tree call");
        let tree: serde_json::Value =
            serde_json::from_str(tree_body.split(" || ").nth(1).unwrap()).unwrap();
        assert_eq!(
            tree["base_tree"], "BASETREE",
            "tree must overlay the base commit's tree"
        );
        let entries = tree["tree"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        let text = entries.iter().find(|e| e["path"] == "a.txt").unwrap();
        assert_eq!(text["content"], "hello");
        assert_eq!(text["mode"], "100644");
        let bin = entries.iter().find(|e| e["path"] == "bin.dat").unwrap();
        assert_eq!(bin["sha"], "BLOB1");
        assert!(bin.get("content").is_none());

        let commit_body = seen
            .iter()
            .find(|s| s.contains("POST ") && s.contains("/git/commits"))
            .expect("commit call");
        let commit: serde_json::Value =
            serde_json::from_str(commit_body.split(" || ").nth(1).unwrap()).unwrap();
        assert_eq!(commit["tree"], "TREE1");
        assert_eq!(commit["parents"], serde_json::json!(["TIP"]));

        let ref_body = seen
            .iter()
            .find(|s| s.contains("POST ") && s.contains("/git/refs"))
            .expect("ref call");
        let refr: serde_json::Value =
            serde_json::from_str(ref_body.split(" || ").nth(1).unwrap()).unwrap();
        assert_eq!(refr["ref"], "refs/heads/pilot-scaffold");
        assert_eq!(refr["sha"], "COMMIT1");
    }

    /// Retry after a partial success: when the branch ref already exists,
    /// POST returns 422 and the push must fall back to a non-force PATCH
    /// instead of failing. Text-only files, so no blob call.
    #[tokio::test]
    async fn pilot_data_api_push_falls_back_to_patch_on_existing_ref() {
        use std::sync::{Arc, Mutex};
        use tokio::io::AsyncWriteExt;

        let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let seen_srv = seen.clone();
        let server = tokio::spawn(async move {
            for _ in 0..6 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let req = read_request(&mut stream).await;
                let head = req.lines().next().unwrap_or("").to_owned();
                let body = req.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
                seen_srv.lock().unwrap().push(format!("{head} || {body}"));
                let (status, payload) =
                    if head.starts_with("GET ") && head.contains("/git/ref/heads/") {
                        (200, "{\"object\":{\"sha\":\"TIP\"}}".to_string())
                    } else if head.starts_with("GET ") && head.contains("/git/commits/") {
                        (200, "{\"tree\":{\"sha\":\"BASETREE\"}}".to_string())
                    } else if head.starts_with("POST ") && head.contains("/git/trees") {
                        (200, "{\"sha\":\"TREE1\"}".to_string())
                    } else if head.starts_with("POST ") && head.contains("/git/commits") {
                        (200, "{\"sha\":\"COMMIT1\"}".to_string())
                    } else if head.starts_with("POST ") && head.contains("/git/refs") {
                        (
                            422,
                            "{\"message\":\"Reference already exists\"}".to_string(),
                        )
                    } else if head.starts_with("PATCH ") && head.contains("/git/refs/heads/") {
                        assert!(
                            head.contains("/repos/o/r/git/refs/heads/pilot-scaffold"),
                            "patch path: {head}"
                        );
                        (200, "{\"ref\":\"refs/heads/pilot-scaffold\"}".to_string())
                    } else {
                        panic!("unexpected mock request: {head}");
                    };
                let resp = format!(
                    "HTTP/1.1 {status} {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    if status == 200 { "OK" } else { "Unprocessable Entity" },
                    payload.len(),
                    payload
                );
                stream.write_all(resp.as_bytes()).await.unwrap();
            }
        });

        let http = reqwest::Client::builder().build().unwrap();
        let github = GithubClient {
            http,
            api_base: format!("http://{addr}"),
            org: "o".to_string(),
            token: "DUMMY".to_string(),
        };
        github
            .push_rendered_via_api("r", &[("a.txt".to_string(), b"hello".to_vec())], "trunk")
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), server)
            .await
            .expect("mock server hung")
            .unwrap();

        let seen = seen.lock().unwrap();
        let patch_body = seen
            .iter()
            .find(|s| s.contains("PATCH ") && s.contains("/git/refs/heads/"))
            .expect("patch call");
        let patch: serde_json::Value =
            serde_json::from_str(patch_body.split(" || ").nth(1).unwrap()).unwrap();
        assert_eq!(patch["sha"], "COMMIT1");
        assert_eq!(patch["force"], false);
    }

    /// Regression test for the pilot-consumer-gen PR#1 CI double-red:
    /// 1. `backend-test-cmd` with `--manifest-path backend/...` fails because
    ///    L1 pr-ci runs it under `working-directory: backend`.
    /// 2. `.issue-resolver.yml` missed the v1.0.20 contract sections
    ///    (`pipeline_test.*`, `deploy.*`) so pipeline-contract-check fails.
    /// Renders the real on-disk template so drift is caught here, not in CI.
    #[test]
    fn pilot_template_satisfies_l1_contract() {
        let dir = match template_dir() {
            Ok(d) => d,
            Err(_) => {
                eprintln!("template dir absent, skipping pilot_template_satisfies_l1_contract");
                return;
            }
        };
        let files = collect_template_files(&dir).expect("template dir readable");
        let rendered = render_all(&files, &fixture_inputs());
        let get = |name: &str| {
            let (_, bytes) = rendered
                .iter()
                .find(|(rel, _)| rel == name)
                .unwrap_or_else(|| panic!("template missing {name}"));
            String::from_utf8(bytes.clone()).expect("template utf8")
        };

        let on_pr = get(".github/workflows/on-pr.yml");
        let cmd_line = on_pr
            .lines()
            .find(|l| l.contains("backend-test-cmd:"))
            .expect("on-pr sets backend-test-cmd");
        assert!(
            !cmd_line.contains("--manifest-path"),
            "backend-test-cmd runs under working-directory backend: {cmd_line}"
        );

        let resolver = get(".issue-resolver.yml");
        assert!(
            !has_unrendered_placeholders(&resolver),
            "unrendered placeholder survived"
        );
        // v1.0.20 pipeline-contract-check required leaves.
        for section in [
            "pipeline_test:",
            "auto_merge:",
            "human_review:",
            "discussion:",
            "category:",
            "deploy:",
            "health_endpoint:",
        ] {
            assert!(resolver.contains(section), "contract field missing: {section}");
        }
        assert!(
            resolver.matches("title_template:").count() >= 2,
            "auto_merge + human_review each need a title_template"
        );
        assert!(
            resolver.matches("body_template:").count() >= 2,
            "auto_merge + human_review each need a body_template"
        );
        assert!(
            resolver.contains("https://pilot-api.xieyucheng.top"),
            "deploy.url must render the pilot api url"
        );
    }
}
