//! Pilot consumer provisioner (pilot plan Task 4).
//!
//! `provisionPilotConsumer(spaceId, templateVersion)` runs a minimal closed
//! loop: create repo via GitHub API -> render `templates/pilot-consumer/`
//! (substituting the five `__PILOT_*` placeholders) -> push a scaffold branch
//! -> open the initial PR. If any step after repo creation fails, the repo is
//! deleted again (fail-closed rollback).
//!
//! Live provisioning needs Task 3 credentials (`PILOT_GITHUB_TOKEN` or
//! `GITHUB_TOKEN` plus `PILOT_GITHUB_ORG`); without them the mutation fails
//! fast before creating anything.

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
pub const PLACEHOLDER_OSS_BUCKET: &str = "__PILOT_OSS_BUCKET__";
pub const PLACEHOLDER_RUNNER: &str = "__PILOT_RUNNER__";
pub const PLACEHOLDER_FRONTEND_URL: &str = "__PILOT_FRONTEND_URL__";
pub const PLACEHOLDER_API_URL: &str = "__PILOT_API_URL__";

/// Branch carrying the rendered scaffold; the initial PR targets `main`.
pub const SCAFFOLD_BRANCH: &str = "pilot-scaffold";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotInputs {
    pub repo_name: String,
    pub oss_bucket: String,
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
        "GitHub credentials missing: set PILOT_GITHUB_TOKEN (or GITHUB_TOKEN) and PILOT_GITHUB_ORG"
    )]
    MissingCredentials,
    #[error("create repo failed: {0}")]
    CreateRepo(String),
    #[error("render failed: {0}")]
    Render(String),
    #[error("push failed: {0}")]
    Push(String),
    #[error("open PR failed: {0}")]
    OpenPr(String),
}

/// Substitute the five `__PILOT_*` placeholders in one template file.
pub fn render_content(template: &str, inputs: &PilotInputs) -> String {
    template
        .replace(PLACEHOLDER_REPO, &inputs.repo_name)
        .replace(PLACEHOLDER_OSS_BUCKET, &inputs.oss_bucket)
        .replace(PLACEHOLDER_RUNNER, &inputs.runner)
        .replace(PLACEHOLDER_FRONTEND_URL, &inputs.frontend_url)
        .replace(PLACEHOLDER_API_URL, &inputs.api_url)
}

/// True when any of the five `__PILOT_*` placeholders survived rendering.
/// Only exact placeholders count: bare `__PILOT__` mentions in comments
/// (e.g. `.issue-resolver.yml`) are documentation, not render targets.
pub fn has_unrendered_placeholders(content: &str) -> bool {
    [
        PLACEHOLDER_REPO,
        PLACEHOLDER_OSS_BUCKET,
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
        oss_bucket: std::env::var("PILOT_OSS_BUCKET")
            .unwrap_or_else(|_| format!("pilot-frontend-{short}")),
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

impl GithubClient {
    fn from_env() -> Result<Self, PilotError> {
        let token = std::env::var("PILOT_GITHUB_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .map_err(|_| PilotError::MissingCredentials)?;
        if token.trim().is_empty() {
            return Err(PilotError::MissingCredentials);
        }
        let org = std::env::var("PILOT_GITHUB_ORG").map_err(|_| PilotError::MissingCredentials)?;
        if org.trim().is_empty() {
            return Err(PilotError::MissingCredentials);
        }
        let api_base = std::env::var("GITHUB_API_URL")
            .unwrap_or_else(|_| "https://api.github.com".to_string());
        let http = reqwest::Client::builder()
            .user_agent("eap-pilot-provisioner")
            .build()
            .map_err(|e| PilotError::CreateRepo(e.to_string()))?;
        Ok(Self {
            http,
            api_base: api_base.trim_end_matches('/').to_owned(),
            org,
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
async fn push_rendered(
    rendered: &[(String, Vec<u8>)],
    repo_name: &str,
    org: &str,
    token: &str,
    redacted_token: &str,
) -> Result<(), PilotError> {
    let workdir =
        std::env::temp_dir().join(format!("pilot-{repo_name}-{}", Uuid::new_v4().simple()));
    let workdir_clone = workdir.clone();
    let files: Vec<(String, Vec<u8>)> = rendered.to_vec();
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

    let remote = format!("https://x-access-token:{token}@github.com/{org}/{repo_name}.git");
    let redact = |s: String| s.replace(token, redacted_token);
    let git = async |args: &[&str]| -> Result<String, PilotError> {
        // The `remote add` args embed the token URL, so the echoed command
        // itself must be redacted, not just git's output.
        let cmd = redact(args.join(" "));
        let out = tokio::process::Command::new("git")
            .args(args)
            .current_dir(&workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .await
            .map_err(|e| PilotError::Push(format!("git {cmd} failed: {e}")))?;
        if !out.status.success() {
            let stderr = redact(String::from_utf8_lossy(&out.stderr).into_owned());
            let stdout = redact(String::from_utf8_lossy(&out.stdout).into_owned());
            return Err(PilotError::Push(format!(
                "git {cmd} failed: {stderr} {stdout}"
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    };

    let result = async {
        git(&["init", "-b", SCAFFOLD_BRANCH]).await?;
        git(&["config", "user.email", "pilot-generator@eap.local"]).await?;
        git(&["config", "user.name", "eap-pilot-generator"]).await?;
        git(&["add", "-A"]).await?;
        git(&["commit", "-m", "chore: pilot scaffold"]).await?;
        git(&["remote", "add", "origin", &remote]).await?;
        git(&["push", "-u", "origin", SCAFFOLD_BRANCH]).await?;
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
    let github = GithubClient::from_env()?;

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
            oss_bucket: "pilot-frontend-xyc".to_string(),
            runner: "eap-backend".to_string(),
            frontend_url: "https://pilot.xieyucheng.top".to_string(),
            api_url: "https://pilot-api.xieyucheng.top".to_string(),
        }
    }

    #[test]
    fn pilot_render_snapshot() {
        let inputs = fixture_inputs();
        let template = "ghcr-image: ghcr.io/link-seek/__PILOT_REPO__\n\
             oss-bucket: __PILOT_OSS_BUCKET__\n\
             runner-label: __PILOT_RUNNER__\n\
             frontend-url: __PILOT_FRONTEND_URL__\n\
             api-url: __PILOT_API_URL__\n";
        let expected = "ghcr-image: ghcr.io/link-seek/pilot-demo\n\
             oss-bucket: pilot-frontend-xyc\n\
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
            PLACEHOLDER_OSS_BUCKET,
            PLACEHOLDER_RUNNER,
            PLACEHOLDER_FRONTEND_URL,
            PLACEHOLDER_API_URL,
        ]
        .join("|");
        let rendered = render_content(&template, &inputs);
        assert_eq!(
            rendered,
            "pilot-demo|pilot-frontend-xyc|eap-backend|https://pilot.xieyucheng.top|https://pilot-api.xieyucheng.top"
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
        assert!(text.contains("pilot-frontend-xyc"));
    }

    #[test]
    fn pilot_render_bytes_passthrough_binary() {
        let inputs = fixture_inputs();
        let binary = vec![0xff, 0xfe, 0x00, 0x01];
        assert_eq!(render_bytes(&binary, &inputs), binary);
    }
}
