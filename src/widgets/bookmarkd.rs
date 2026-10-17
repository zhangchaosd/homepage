//! Pinned bookmarks from bookmarkd, read with a read-only API token:
//! `GET /api/v1/bookmarks?pinned=true` with `Authorization: Bearer <token>`.
//!
//! bookmarkd only answers requests whose `Host` is its public origin, so when
//! the API is reached at a LAN address the public host from `link` is sent
//! instead, just as the reverse proxy would.

use serde::{Deserialize, Serialize};

use super::{FetchResult, get_json};
use crate::config::Bookmarkd;

#[derive(Deserialize)]
struct Page {
    items: Vec<Bookmark>,
}

#[derive(Deserialize)]
struct Bookmark {
    id: String,
    url_raw: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    pinned_position: i64,
    icon: Option<String>,
}

#[derive(Serialize)]
struct BookmarkView {
    id: String,
    title: String,
    url: String,
    host: String,
    tags: Vec<String>,
    icon: Option<String>,
}

pub async fn fetch(client: &reqwest::Client, config: &Bookmarkd) -> FetchResult {
    let limit = config.limit.to_string();
    let mut request = client
        .get(format!("{}/api/v1/bookmarks", config.url))
        .bearer_auth(&config.token)
        .query(&[("pinned", "true"), ("limit", limit.as_str())]);
    if let Some(host) = public_host(&config.url, &config.link) {
        request = request.header(reqwest::header::HOST, host);
    }
    let page: Page = get_json(request, "bookmarkd").await?;
    let mut items = page.items;
    items.sort_by_key(|bookmark| bookmark.pinned_position);
    let views: Vec<BookmarkView> = items
        .into_iter()
        .filter(|bookmark| is_web_url(&bookmark.url_raw))
        .take(config.limit)
        .map(|bookmark| {
            let host = url::Url::parse(&bookmark.url_raw)
                .ok()
                .and_then(|url| {
                    url.host_str()
                        .map(|host| host.trim_start_matches("www.").to_owned())
                })
                .unwrap_or_default();
            BookmarkView {
                id: bookmark.id,
                title: if bookmark.title.trim().is_empty() {
                    host.clone()
                } else {
                    bookmark.title
                },
                url: bookmark.url_raw,
                host,
                tags: bookmark.tags,
                // Only inline raster/vector images; anything else falls back to a monogram.
                icon: bookmark.icon.filter(|icon| icon.starts_with("data:image/")),
            }
        })
        .collect();
    Ok(serde_json::json!({ "items": views, "link": config.link }))
}

/// The `host[:port]` of `link` when it differs from the one in `url`.
fn public_host(url: &str, link: &str) -> Option<String> {
    let authority = |value: &str| {
        let parsed = url::Url::parse(value).ok()?;
        let host = parsed.host_str()?.to_owned();
        Some(match parsed.port() {
            Some(port) => format!("{host}:{port}"),
            None => host,
        })
    };
    let public = authority(link)?;
    (authority(url)? != public).then_some(public)
}

/// Links are rendered as `<a href>`, so `javascript:` and friends are dropped.
fn is_web_url(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_the_public_host_for_lan_urls() {
        assert_eq!(
            public_host("http://192.168.1.10:8765", "https://bookmark.example.com").as_deref(),
            Some("bookmark.example.com")
        );
        assert_eq!(
            public_host("http://10.0.0.2:8765", "http://localhost:8765").as_deref(),
            Some("localhost:8765")
        );
        assert_eq!(
            public_host(
                "https://bookmark.example.com",
                "https://bookmark.example.com"
            ),
            None
        );
    }

    #[test]
    fn only_web_links_are_shown() {
        assert!(is_web_url("https://example.com/a"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("not a url"));
    }
}
