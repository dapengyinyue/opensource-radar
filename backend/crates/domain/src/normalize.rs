//! 去重键与字段归一——系统正确性命门。
//!
//! 算法（按优先级回落，确定性强）：
//! 1. 提取 GitHub 身份：GitHub 源直接取 `full_name`；HN 源解析 `linked_url`。
//! 2. 得到 GitHub 身份 → `dedup_key = "gh:{owner}/{repo}"`（全小写）。
//!    于是 GitHub 源 repo 与 HN 指向同一 repo 的故事产出相同键 → 合并同一项目行。
//! 3. 非 GitHub 外链 → `dedup_key = "url:" + normalize_url(...)`。
//! 4. 无外链（Ask HN / Show HN）→ `dedup_key = "hn:{object_id}"`。

use std::sync::OnceLock;

use regex::Regex;

use crate::models::{NormalizedRecord, SourceKind};
use crate::source::{GithubRepoRaw, HnStoryRaw, RawItem};

fn github_repo_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^https?://(?:www\.)?github\.com/([^/]+)/([^/#?]+?)(?:\.git)?(?:[/?#].*)?$")
            .expect("valid github regex")
    })
}

/// 从 URL 提取 GitHub `owner/repo`（全小写）。非 GitHub 仓库链接返回 None。
pub fn extract_github_identity(url: &str) -> Option<(String, String)> {
    let caps = github_repo_re().captures(url.trim())?;
    let owner = caps.get(1)?.as_str().to_ascii_lowercase();
    let repo = caps.get(2)?.as_str().to_ascii_lowercase();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

/// 规范化非 GitHub URL：小写 host（去 `www.`）+ 小写 path（去尾斜杠、去 query/fragment）。
pub fn normalize_url(url: &str) -> String {
    let url = url.trim();
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let (authority, path_part) = rest.split_once('/').unwrap_or((rest, ""));
    let path = path_part
        .split_once(['?', '#'])
        .map(|(p, _)| p)
        .unwrap_or(path_part);
    let host = authority.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let path_lower = path.to_ascii_lowercase();
    let path = path_lower.trim_end_matches('/');
    if path.is_empty() {
        host.to_string()
    } else {
        format!("{host}/{path}")
    }
}

/// 拆分 `owner/repo` 为全小写元组。无 `/` 时 owner 为空、repo 为整串。
fn split_full_name(full_name: &str) -> (String, String) {
    match full_name.split_once('/') {
        Some((o, r)) => (o.to_ascii_lowercase(), r.to_ascii_lowercase()),
        None => (String::new(), full_name.to_ascii_lowercase()),
    }
}

/// 将原始项归一为可入库的 `NormalizedRecord`。
pub fn normalize(item: &RawItem) -> NormalizedRecord {
    match item {
        RawItem::GithubRepo(r) => normalize_github(r),
        RawItem::HnStory(s) => normalize_hn(s),
    }
}

fn normalize_github(r: &GithubRepoRaw) -> NormalizedRecord {
    let (owner, repo) = split_full_name(&r.full_name);
    let dedup_key = format!("gh:{owner}/{repo}");
    NormalizedRecord {
        dedup_key,
        name: r.name.clone(),
        full_name: Some(r.full_name.to_ascii_lowercase()),
        description: r.description.clone(),
        repo_url: Some(r.html_url.clone()),
        homepage_url: r
            .homepage
            .clone()
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty()),
        language: r.language.clone(),
        topics: r.topics.clone(),
        stars: Some(r.stargazers_count),
        forks: Some(r.forks_count),
        open_issues: Some(r.open_issues_count),
        hn_points: None,
        hn_comment_count: None,
        github_created_at: Some(r.created_at),
        github_updated_at: Some(r.updated_at),
        last_activity_at: Some(r.updated_at),
        source_kind: SourceKind::Github,
        metadata: r.extra.clone(),
    }
}

fn normalize_hn(s: &HnStoryRaw) -> NormalizedRecord {
    // GitHub 身份优先：HN 指向 GitHub repo → 与 GitHub 源合并
    if let Some(linked) = s.linked_url.as_deref() {
        if let Some((owner, repo)) = extract_github_identity(linked) {
            return NormalizedRecord {
                dedup_key: format!("gh:{owner}/{repo}"),
                name: s.title.clone(),
                full_name: Some(format!("{owner}/{repo}")),
                description: Some(s.title.clone()),
                repo_url: Some(format!("https://github.com/{owner}/{repo}")),
                homepage_url: None,
                language: None,
                topics: vec![],
                stars: None,
                forks: None,
                open_issues: None,
                hn_points: s.points,
                hn_comment_count: s.comment_count,
                github_created_at: None,
                github_updated_at: None,
                last_activity_at: s.posted_at,
                source_kind: SourceKind::Hackernews,
                metadata: s.extra.clone(),
            };
        }
        // 非 GitHub 外链
        return NormalizedRecord {
            dedup_key: format!("url:{}", normalize_url(linked)),
            name: s.title.clone(),
            full_name: None,
            description: Some(s.title.clone()),
            repo_url: Some(linked.to_string()),
            homepage_url: None,
            language: None,
            topics: vec![],
            stars: None,
            forks: None,
            open_issues: None,
            hn_points: s.points,
            hn_comment_count: s.comment_count,
            github_created_at: None,
            github_updated_at: None,
            last_activity_at: s.posted_at,
            source_kind: SourceKind::Hackernews,
            metadata: s.extra.clone(),
        };
    }
    // 无外链
    NormalizedRecord {
        dedup_key: format!("hn:{}", s.object_id),
        name: s.title.clone(),
        full_name: None,
        description: Some(s.title.clone()),
        repo_url: None,
        homepage_url: None,
        language: None,
        topics: vec![],
        stars: None,
        forks: None,
        open_issues: None,
        hn_points: s.points,
        hn_comment_count: s.comment_count,
        github_created_at: None,
        github_updated_at: None,
        last_activity_at: s.posted_at,
        source_kind: SourceKind::Hackernews,
        metadata: s.extra.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn ts() -> DateTime<Utc> {
        "2026-01-01T00:00:00Z".parse().unwrap()
    }

    fn gh() -> GithubRepoRaw {
        GithubRepoRaw {
            full_name: "Tokio-Rs/tokio".into(),
            name: "tokio".into(),
            description: Some("async runtime".into()),
            html_url: "https://github.com/tokio-rs/tokio".into(),
            homepage: Some("https://tokio.rs".into()),
            language: Some("Rust".into()),
            topics: vec!["async".into(), "runtime".into()],
            stargazers_count: 27000,
            forks_count: 3000,
            open_issues_count: 400,
            created_at: ts(),
            updated_at: ts(),
            node_id: Some("N1".into()),
            extra: serde_json::json!({}),
        }
    }

    fn hn(linked: Option<&str>, object_id: &str, title: &str) -> HnStoryRaw {
        HnStoryRaw {
            object_id: object_id.into(),
            hn_url: format!("https://news.ycombinator.com/item?id={object_id}"),
            linked_url: linked.map(String::from),
            title: title.into(),
            author: Some("alice".into()),
            points: Some(120),
            comment_count: Some(45),
            posted_at: Some(ts()),
            extra: serde_json::json!({}),
        }
    }

    #[test]
    fn github_source_dedup_key() {
        let r = normalize(&RawItem::GithubRepo(gh()));
        assert_eq!(r.dedup_key, "gh:tokio-rs/tokio");
        assert_eq!(r.full_name.as_deref(), Some("tokio-rs/tokio"));
        assert_eq!(r.stars, Some(27000));
        assert_eq!(r.language.as_deref(), Some("Rust"));
        assert_eq!(r.source_kind, SourceKind::Github);
        assert_eq!(r.last_activity_at, Some(ts()));
    }

    #[test]
    fn hn_linking_github_repo_merges_key() {
        let s = hn(
            Some("https://github.com/Tokio-Rs/tokio/issues/1"),
            "111",
            "Tokio discussion",
        );
        let r = normalize(&RawItem::HnStory(s));
        assert_eq!(r.dedup_key, "gh:tokio-rs/tokio");
        assert_eq!(r.full_name.as_deref(), Some("tokio-rs/tokio"));
        assert_eq!(
            r.repo_url.as_deref(),
            Some("https://github.com/tokio-rs/tokio")
        );
        assert_eq!(r.hn_points, Some(120));
        assert_eq!(r.stars, None);
        assert_eq!(r.source_kind, SourceKind::Hackernews);
    }

    #[test]
    fn hn_linking_github_with_git_suffix() {
        let s = hn(Some("https://github.com/Owner/Repo.git"), "112", "Repo");
        let r = normalize(&RawItem::HnStory(s));
        assert_eq!(r.dedup_key, "gh:owner/repo");
    }

    #[test]
    fn hn_linking_github_with_www_and_case() {
        let s = hn(Some("http://www.GitHub.com/Owner/Repo"), "113", "Repo");
        let r = normalize(&RawItem::HnStory(s));
        assert_eq!(r.dedup_key, "gh:owner/repo");
    }

    #[test]
    fn hn_non_github_external_link() {
        let s = hn(
            Some("https://crates.io/crates/axum/?version=0.7"),
            "114",
            "Axum crate",
        );
        let r = normalize(&RawItem::HnStory(s));
        assert_eq!(r.dedup_key, "url:crates.io/crates/axum");
        assert_eq!(
            r.repo_url.as_deref(),
            Some("https://crates.io/crates/axum/?version=0.7")
        );
        assert_eq!(r.hn_points, Some(120));
    }

    #[test]
    fn hn_no_link_uses_object_id() {
        let s = hn(None, "999", "Ask HN: best Rust framework?");
        let r = normalize(&RawItem::HnStory(s));
        assert_eq!(r.dedup_key, "hn:999");
        assert_eq!(r.name, "Ask HN: best Rust framework?");
        assert!(r.repo_url.is_none());
    }

    #[test]
    fn github_and_hn_to_same_repo_share_key() {
        let g = normalize(&RawItem::GithubRepo(gh()));
        let h = normalize(&RawItem::HnStory(hn(
            Some("https://github.com/tokio-rs/tokio"),
            "1",
            "x",
        )));
        assert_eq!(g.dedup_key, h.dedup_key);
    }

    #[test]
    fn normalize_url_strips_query_fragment_www_trailing_slash() {
        assert_eq!(
            normalize_url("https://www.Crates.io/crates/axum/"),
            "crates.io/crates/axum"
        );
        assert_eq!(
            normalize_url("https://example.com/path?q=1#frag"),
            "example.com/path"
        );
        assert_eq!(normalize_url("http://www.example.com"), "example.com");
        assert_eq!(normalize_url("example.com/"), "example.com");
    }

    #[test]
    fn extract_github_identity_rejects_non_repo() {
        assert_eq!(
            extract_github_identity("https://github.com/tokio-rs/tokio"),
            Some(("tokio-rs".into(), "tokio".into()))
        );
        assert_eq!(extract_github_identity("https://gitlab.com/a/b"), None);
        assert_eq!(extract_github_identity("not a url"), None);
    }
}
