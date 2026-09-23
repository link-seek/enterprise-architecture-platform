//! Pilot consumer provisioner (pilot plan Task 4).
//!
//! `provisionPilotConsumer(spaceId, templateVersion)` runs a minimal closed
//! loop: create repo via GitHub API -> render `templates/pilot-consumer/`
//! (substituting the four `__PILOT_*` placeholders) -> push a scaffold branch
//! -> open the initial PR. If any step after repo creation fails, the repo is
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

    async fn create_repo(&self, name: &str) -> Result<String, PilotError> {
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
        body.get("html_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
            .ok_or_else(|| PilotError::CreateRepo("missing html_url".to_string()))
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

    async fn open_pr(&self, repo: &str, template_version: &str) -> Result<String, PilotError> {
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
                "base": "main",
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

/// Push rendered files as `SCAFFOLD_BRANCH` via the git CLI. The token travels
/// only in the remote URL of a temp repo; failures redact it before surfacing.
/// Run one git command inside `workdir`, redacting the push URL (which
/// embeds the token) from any surfaced command/error text.
async fn git_in(
    workdir: &std::path::Path,
    remote: &str,
    redact_remote: &str,
    args: &[&str],
) -> Result<String, PilotError> {
    // The `remote add` args embed the token URL, so the echoed command
    // itself must be redacted, not just git's output.
    let cmd = args.join(" ").replace(remote, redact_remote);
    let out = tokio::process::Command::new("git")
        .args(args)
        .current_dir(workdir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .map_err(|e| PilotError::Push(format!("git {cmd} failed: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr)
            .into_owned()
            .replace(remote, redact_remote);
        let stdout = String::from_utf8_lossy(&out.stdout)
            .into_owned()
            .replace(remote, redact_remote);
        return Err(PilotError::Push(format!(
            "git {cmd} failed: {stderr} {stdout}"
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Set up the scaffold branch cut from the remote `main` tip (create_repo
/// uses `auto_init`, so `main` always exists with an initial commit).
/// Rendered files must be written AFTER this step: checking out `main`'s
/// tree would refuse to overwrite same-named scaffold files (e.g. the
/// template's own README.md vs auto_init's README.md).
async fn prepare_scaffold_branch(
    workdir: &std::path::Path,
    remote: &str,
    redact_remote: &str,
) -> Result<(), PilotError> {
    git_in(workdir, remote, redact_remote, &["init", "-b", SCAFFOLD_BRANCH]).await?;
    git_in(
        workdir,
        remote,
        redact_remote,
        &["config", "user.email", "pilot-generator@eap.local"],
    )
    .await?;
    git_in(
        workdir,
        remote,
        redact_remote,
        &["config", "user.name", "eap-pilot-generator"],
    )
    .await?;
    git_in(
        workdir,
        remote,
        redact_remote,
        &["remote", "add", "origin", remote],
    )
    .await?;
    git_in(workdir, remote, redact_remote, &["fetch", "origin", "main"]).await?;
    // Cut the scaffold branch from main's tip so the follow-up open_pr has
    // common history; without this it fails with
    // `422 ... no history in common`.
    git_in(
        workdir,
        remote,
        redact_remote,
        &["checkout", "-B", SCAFFOLD_BRANCH, "origin/main"],
    )
    .await?;
    Ok(())
}

/// Commit everything in `workdir` onto the scaffold branch and push.
async fn commit_and_push_scaffold(
    workdir: &std::path::Path,
    remote: &str,
    redact_remote: &str,
) -> Result<(), PilotError> {
    git_in(workdir, remote, redact_remote, &["add", "-A"]).await?;
    git_in(
        workdir,
        remote,
        redact_remote,
        &["commit", "-m", "chore: pilot scaffold"],
    )
    .await?;
    git_in(
        workdir,
        remote,
        redact_remote,
        &["push", "-u", "origin", SCAFFOLD_BRANCH],
    )
    .await?;
    Ok(())
}

async fn push_rendered(
    rendered: &[(String, Vec<u8>)],
    repo_name: &str,
    org: &str,
    token: &str,
    redacted_token: &str,
) -> Result<(), PilotError> {
    let workdir =
        std::env::temp_dir().join(format!("pilot-{repo_name}-{}", Uuid::new_v4().simple()));
    std::fs::create_dir_all(&workdir).map_err(|e| PilotError::Push(e.to_string()))?;
    let workdir_clone = workdir.clone();
    let files: Vec<(String, Vec<u8>)> = rendered.to_vec();
    let remote = format!("https://x-access-token:{token}@github.com/{org}/{repo_name}.git");

    let result = async {
        // Order matters: branch off main first (checkout populates main's
        // tree), then overlay the rendered files, then commit + push.
        prepare_scaffold_branch(&workdir, &remote, redacted_token).await?;
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            for (rel, bytes) in &files {
                let dest = workdir_clone.join(rel);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&dest, bytes).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .await
        .map_err(|e| PilotError::Push(e.to_string()))?
        .map_err(PilotError::Push)?;
        commit_and_push_scaffold(&workdir, &remote, redacted_token).await?;
        Ok::<(), PilotError>(())
    }
    .await;

    let _ = tokio::fs::remove_dir_all(&workdir).await;
    result
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

    let repo_url = github.create_repo(&inputs.repo_name).await?;

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
        push_rendered(
            &rendered,
            &inputs.repo_name,
            &github.org,
            &github.token,
            "<redacted>",
        )
        .await?;
        let pr_url = github.open_pr(&inputs.repo_name, &template_version).await?;
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

    /// Regression test for the live `422 ... no history in common` failure:
    /// the scaffold branch must be cut from the remote `main` tip. Local
    /// bare repos only, no network. Skips gracefully when `git` is missing.
    #[tokio::test]
    async fn pilot_push_shares_history_with_main() {
        use std::process::Command as SyncCommand;

        if SyncCommand::new("git")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("git not available, skipping pilot_push_shares_history_with_main");
            return;
        }
        fn git(args: &[&str], dir: &std::path::Path) {
            let out = SyncCommand::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .expect("git command failed to run");
            assert!(
                out.status.success(),
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }

        let base = std::env::temp_dir().join(format!("pilot-test-{}", Uuid::new_v4().simple()));
        let origin = base.join("origin.git");
        let seed = base.join("seed");
        let scaffold = base.join("scaffold");
        std::fs::create_dir_all(&seed).unwrap();

        // Simulate create_repo(auto_init=true): remote `main` with one
        // commit, including a README.md that also exists in the template.
        SyncCommand::new("git")
            .args(["init", "--bare", "-b", "main"])
            .arg(&origin)
            .output()
            .unwrap();
        git(&["init", "-b", "main"], &seed);
        git(&["config", "user.email", "t@t"], &seed);
        git(&["config", "user.name", "t"], &seed);
        std::fs::write(seed.join("README.md"), "remote initial\n").unwrap();
        git(&["add", "-A"], &seed);
        git(&["commit", "-m", "initial"], &seed);
        let remote = origin.display().to_string();
        git(&["remote", "add", "origin", &remote], &seed);
        git(&["push", "origin", "main"], &seed);

        // Production order: prepare branch first, then overlay rendered
        // files (including the colliding README.md), then commit + push.
        std::fs::create_dir_all(&scaffold).unwrap();
        prepare_scaffold_branch(&scaffold, &remote, "REDACTED")
            .await
            .unwrap();
        std::fs::write(scaffold.join("README.md"), "scaffold version\n").unwrap();
        std::fs::write(scaffold.join("extra.txt"), "x\n").unwrap();
        commit_and_push_scaffold(&scaffold, &remote, "REDACTED")
            .await
            .unwrap();

        // Scaffold branch must share history with main ...
        let ancestor = SyncCommand::new("git")
            .arg("--git-dir")
            .arg(&origin)
            .args(["merge-base", "--is-ancestor", "main", "pilot-scaffold"])
            .output()
            .unwrap();
        assert!(
            ancestor.status.success(),
            "pilot-scaffold shares no history with main"
        );
        // ... and the scaffold content must win over main's tree.
        let readme = SyncCommand::new("git")
            .arg("--git-dir")
            .arg(&origin)
            .args(["show", "pilot-scaffold:README.md"])
            .output()
            .unwrap();
        assert!(readme.status.success());
        assert_eq!(
            String::from_utf8_lossy(&readme.stdout),
            "scaffold version\n"
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
