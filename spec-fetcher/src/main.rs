use anyhow::{bail, Context, Result};
use clap::Parser;
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs;

#[derive(Parser)]
#[command(about = "Fetch Kubernetes OpenAPI v3 specs for a given minor version series")]
struct Cli {
    /// Kubernetes minor version to fetch, e.g. "1.33"
    minor_version: String,

    /// Directory to store specs
    #[arg(long, default_value = "./specs")]
    specs_dir: PathBuf,

    /// GitHub API token (or GITHUB_TOKEN env var)
    #[arg(long, env = "GITHUB_TOKEN")]
    github_token: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    draft: bool,
}

#[derive(Deserialize)]
struct ContentEntry {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    download_url: Option<String>,
}

fn patch_number(tag: &str) -> Option<u64> {
    let s = tag.strip_prefix('v')?;
    let mut parts = s.splitn(3, '.');
    parts.next()?;
    parts.next()?;
    parts.next()?.parse().ok()
}

fn minor_of(tag: &str) -> u64 {
    let s = tag.strip_prefix('v').unwrap_or(tag);
    s.split('.').nth(1).and_then(|m| m.parse().ok()).unwrap_or(0)
}

fn build_client(token: Option<&str>) -> Result<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        "kubernetools-spec-fetcher/0.1".parse()?,
    );
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github.v3+json".parse()?,
    );
    if let Some(t) = token {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {t}").parse()?,
        );
    }
    Ok(Client::builder().default_headers(headers).build()?)
}

async fn find_latest_release(client: &Client, prefix: &str, api_base: &str) -> Result<String> {
    let target_minor = minor_of(prefix.trim_end_matches('.'));
    let mut best: Option<(u64, String)> = None;

    for page in 1u32..=10 {
        let url = format!(
            "{api_base}/repos/kubernetes/kubernetes/releases\
             ?per_page=100&page={page}"
        );
        let releases: Vec<Release> = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if releases.is_empty() {
            break;
        }

        for r in &releases {
            if r.prerelease || r.draft {
                continue;
            }
            if !r.tag_name.starts_with(prefix) {
                continue;
            }
            if let Some(p) = patch_number(&r.tag_name) {
                if best.as_ref().map_or(true, |(b, _)| p > *b) {
                    best = Some((p, r.tag_name.clone()));
                }
            }
        }

        // Releases are returned newest-first. Once we've found matches and
        // the last entry on this page is from an older minor, we're past it.
        if best.is_some() && minor_of(&releases.last().unwrap().tag_name) < target_minor {
            break;
        }
    }

    best.map(|(_, tag)| tag)
        .ok_or_else(|| anyhow::anyhow!("no stable release found matching prefix {prefix:?}"))
}

async fn list_spec_files(client: &Client, tag: &str, api_base: &str) -> Result<Vec<ContentEntry>> {
    let url = format!(
        "{api_base}/repos/kubernetes/kubernetes/contents/\
         api/openapi-spec/v3?ref={tag}"
    );
    let entries: Vec<ContentEntry> = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(entries
        .into_iter()
        .filter(|e| e.kind == "file" && e.name.ends_with(".json"))
        .collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let parts: Vec<&str> = cli.minor_version.split('.').collect();
    if parts.len() != 2 || parts[0].parse::<u64>().is_err() || parts[1].parse::<u64>().is_err() {
        bail!(
            "minor_version must be MAJOR.MINOR (e.g. 1.33), got {:?}",
            cli.minor_version
        );
    }

    let client = build_client(cli.github_token.as_deref())?;

    const GITHUB_API: &str = "https://api.github.com";
    let prefix = format!("v{}.", cli.minor_version);
    let latest = find_latest_release(&client, &prefix, GITHUB_API).await?;
    println!("Latest release for {}: {latest}", cli.minor_version);

    let version_dir = cli.specs_dir.join(format!("v{}", cli.minor_version));
    let version_file = version_dir.join(".version");

    if let Ok(stored) = fs::read_to_string(&version_file).await {
        if stored.trim() == latest {
            println!("Already up to date ({latest}), nothing to do.");
            return Ok(());
        }
        println!("Stored: {}, updating to {latest}", stored.trim());
    } else {
        println!("No stored version found, fetching {latest}");
    }

    let files = list_spec_files(&client, &latest, GITHUB_API).await?;
    println!("Downloading {} spec files...", files.len());

    fs::create_dir_all(&version_dir).await?;
    for entry in &files {
        let url = entry
            .download_url
            .as_deref()
            .with_context(|| format!("no download_url for {}", entry.name))?;
        let bytes = client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let dest = version_dir.join(&entry.name);
        fs::write(&dest, &bytes)
            .await
            .with_context(|| format!("writing {}", dest.display()))?;
        println!("  wrote {}", entry.name);
    }

    fs::write(&version_file, &latest).await?;
    println!(
        "Done: stored {latest} ({} files) in {}",
        files.len(),
        version_dir.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── pure unit tests ────────────────────────────────────────────────────

    #[test]
    fn patch_number_valid_tags() {
        assert_eq!(patch_number("v1.33.0"), Some(0));
        assert_eq!(patch_number("v1.33.3"), Some(3));
        assert_eq!(patch_number("v1.33.10"), Some(10));
    }

    #[test]
    fn patch_number_rejects_missing_v_prefix() {
        assert_eq!(patch_number("1.33.3"), None);
    }

    #[test]
    fn patch_number_rejects_too_short() {
        assert_eq!(patch_number("v1.33"), None);
    }

    #[test]
    fn patch_number_rejects_non_numeric_patch() {
        assert_eq!(patch_number("v1.33.x"), None);
        assert_eq!(patch_number("v1.33.0-beta.1"), None);
    }

    #[test]
    fn minor_of_extracts_minor() {
        assert_eq!(minor_of("v1.33.3"), 33);
        assert_eq!(minor_of("v1.36.0"), 36);
    }

    #[test]
    fn minor_of_without_v_prefix() {
        assert_eq!(minor_of("1.33"), 33);
        assert_eq!(minor_of("v1.33"), 33);
    }

    #[test]
    fn minor_of_returns_zero_for_invalid() {
        assert_eq!(minor_of("invalid"), 0);
        assert_eq!(minor_of(""), 0);
    }

    // ── HTTP helpers ───────────────────────────────────────────────────────

    const RELEASES_PATH: &str = "/repos/kubernetes/kubernetes/releases";
    const CONTENTS_PATH: &str =
        "/repos/kubernetes/kubernetes/contents/api/openapi-spec/v3";

    /// Mount a fallback that returns `[]` for every page not covered by a
    /// more-specific mock. Wiremock matches in reverse mount order (LIFO), so
    /// this must be mounted *before* any page-specific mocks.
    async fn mount_empty_fallback(server: &MockServer, path_str: &str) {
        Mock::given(method("GET"))
            .and(path(path_str))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(server)
            .await;
    }

    // ── find_latest_release tests ──────────────────────────────────────────

    #[tokio::test]
    async fn find_latest_release_picks_highest_patch() {
        let server = MockServer::start().await;
        mount_empty_fallback(&server, RELEASES_PATH).await;

        Mock::given(method("GET"))
            .and(path(RELEASES_PATH))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"tag_name": "v1.33.1", "prerelease": false, "draft": false},
                {"tag_name": "v1.33.3", "prerelease": false, "draft": false},
                {"tag_name": "v1.33.0", "prerelease": false, "draft": false},
            ])))
            .mount(&server)
            .await;

        let client = build_client(None).unwrap();
        let tag = find_latest_release(&client, "v1.33.", &server.uri())
            .await
            .unwrap();
        assert_eq!(tag, "v1.33.3");
    }

    #[tokio::test]
    async fn find_latest_release_skips_prerelease_and_draft() {
        let server = MockServer::start().await;
        mount_empty_fallback(&server, RELEASES_PATH).await;

        Mock::given(method("GET"))
            .and(path(RELEASES_PATH))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"tag_name": "v1.33.3", "prerelease": true,  "draft": false},
                {"tag_name": "v1.33.2", "prerelease": false, "draft": true},
                {"tag_name": "v1.33.1", "prerelease": false, "draft": false},
            ])))
            .mount(&server)
            .await;

        let client = build_client(None).unwrap();
        let tag = find_latest_release(&client, "v1.33.", &server.uri())
            .await
            .unwrap();
        assert_eq!(tag, "v1.33.1");
    }

    #[tokio::test]
    async fn find_latest_release_errors_when_no_match() {
        let server = MockServer::start().await;
        mount_empty_fallback(&server, RELEASES_PATH).await;

        Mock::given(method("GET"))
            .and(path(RELEASES_PATH))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"tag_name": "v1.34.0", "prerelease": false, "draft": false},
            ])))
            .mount(&server)
            .await;

        let client = build_client(None).unwrap();
        let result = find_latest_release(&client, "v1.33.", &server.uri()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no stable release"));
    }

    // ── list_spec_files tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn list_spec_files_returns_json_files_only() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(CONTENTS_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"name": "apis__apps__v1_openapi.json", "type": "file",
                 "download_url": "http://example.com/apps.json"},
                {"name": "api__v1_openapi.json", "type": "file",
                 "download_url": "http://example.com/core.json"},
                {"name": "README.md", "type": "file",
                 "download_url": "http://example.com/README.md"},
                {"name": "subdir", "type": "dir", "download_url": null},
            ])))
            .mount(&server)
            .await;

        let client = build_client(None).unwrap();
        let files = list_spec_files(&client, "v1.33.3", &server.uri())
            .await
            .unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.name.ends_with(".json")));
        assert!(files.iter().all(|f| f.kind == "file"));
    }

    #[tokio::test]
    async fn list_spec_files_empty_directory() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(CONTENTS_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let client = build_client(None).unwrap();
        let files = list_spec_files(&client, "v1.33.3", &server.uri())
            .await
            .unwrap();
        assert!(files.is_empty());
    }
}
