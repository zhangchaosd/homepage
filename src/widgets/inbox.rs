//! Cross-device inbox (inboxd): token status for the card, and quick sends
//! with inboxd's write-only script token.
//!
//! A send follows inboxd's ingest contract: read the public capabilities for
//! the instance and data generation, then post one JSON manifest whose
//! `client_submission_id` is also the `Idempotency-Key`. A retry reuses the
//! same id, so a submission can never be stored twice.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{FetchResult, get_json};
use crate::config::Inbox;

const TITLE_MAX: usize = 256;
const TEXT_MAX_BYTES: usize = 1024 * 1024;
const LINKS_MAX: usize = 20;

#[derive(Deserialize)]
struct Me {
    expires_at: Option<String>,
    #[serde(default)]
    scopes: Vec<String>,
}

#[derive(Deserialize)]
struct Capabilities {
    instance_id: String,
    data_generation: String,
}

#[derive(Deserialize)]
struct ReceiptEnvelope {
    receipt: Receipt,
}

#[derive(Deserialize, Serialize)]
pub struct Receipt {
    pub item_id: Option<String>,
    pub status: String,
}

/// Token status shown on the card; sending works independently of it.
pub async fn fetch(client: &reqwest::Client, config: &Inbox) -> FetchResult {
    let me: Me = get_json(
        client
            .get(format!("{}/api/v1/me", config.url))
            .bearer_auth(&config.token),
        "inbox",
    )
    .await?;
    let can_send = me.scopes.iter().any(|scope| scope == "ingest:write");
    Ok(json!({
        "expiresAt": me.expires_at,
        "canSend": can_send,
        "link": config.link,
    }))
}

/// Builds a manifest from free text: `text` is kept as the body and every
/// http(s) address in it also becomes a link.
fn manifest(id: &str, title: Option<&str>, text: &str) -> Result<serde_json::Value, String> {
    let title = title.map(str::trim).filter(|title| !title.is_empty());
    if let Some(title) = title
        && title.chars().count() > TITLE_MAX
    {
        return Err(format!("标题不能超过 {TITLE_MAX} 个字符"));
    }
    let text = text.trim();
    if text.is_empty() {
        return Err("内容不能为空".into());
    }
    if text.len() > TEXT_MAX_BYTES {
        return Err("内容超过 1 MiB".into());
    }
    let links: Vec<_> = extract_links(text)
        .into_iter()
        .map(|url| json!({ "url": url }))
        .collect();
    Ok(json!({
        "schema_version": 1,
        "client_submission_id": id,
        "title": title,
        "body_text": text,
        "links": links,
        "files": [],
        "source": { "kind": "script", "label": "Homepage" },
        "client_captured_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "retention": { "mode": "default" },
    }))
}

/// http(s) addresses in `text`, in order, without duplicates or trailing punctuation.
fn extract_links(text: &str) -> Vec<String> {
    let trailing: &[char] = &['.', ',', ';', ':', '!', '?', ')', ']', '}', '>', '"', '\''];
    // Full-width punctuation never belongs to a typed URL, so it ends one even
    // without a space ("…/a，还有"); other non-ASCII text such as Chinese paths stays.
    let breaks = |c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '<' | '"'
                    | '，'
                    | '。'
                    | '；'
                    | '：'
                    | '！'
                    | '？'
                    | '、'
                    | '（'
                    | '）'
                    | '【'
                    | '】'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '《'
                    | '》'
            )
    };
    let mut links = Vec::new();
    for word in text.split(breaks) {
        let lower = word.to_ascii_lowercase();
        let Some(start) = lower.find("https://").or_else(|| lower.find("http://")) else {
            continue;
        };
        let candidate = word[start..].trim_end_matches(trailing);
        let valid = candidate.len() >= 8
            && candidate.len() <= 8192
            && url::Url::parse(candidate).is_ok_and(|url| url.host_str().is_some());
        if valid && !links.iter().any(|link| link == candidate) {
            links.push(candidate.to_owned());
            if links.len() == LINKS_MAX {
                break;
            }
        }
    }
    links
}

/// Submits one entry. Network failures, server errors and "already in progress"
/// are retried with the same id, which inboxd treats as the same submission.
pub async fn send(
    client: &reqwest::Client,
    config: &Inbox,
    title: Option<&str>,
    text: &str,
) -> Result<Receipt, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let body = manifest(&id, title, text)?;
    let capabilities: Capabilities = get_json(
        client.get(format!("{}/api/v1/capabilities", config.url)),
        "inbox",
    )
    .await?;

    let mut last_error = String::from("收件箱：投递失败");
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(600 * attempt)).await;
        }
        let response = client
            .post(format!("{}/api/v1/ingest", config.url))
            .bearer_auth(&config.token)
            .header("Idempotency-Key", &id)
            .header("X-Inbox-Instance-Id", &capabilities.instance_id)
            .header("X-Inbox-Data-Generation", &capabilities.data_generation)
            .json(&body)
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                last_error = super::describe("收件箱", &error);
                continue;
            }
        };
        let status = response.status();
        if status.is_success() {
            return response
                .json::<ReceiptEnvelope>()
                .await
                .map(|envelope| envelope.receipt)
                .map_err(|_| "收件箱：回执格式异常".into());
        }
        let code = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|body| body["error"]["code"].as_str().map(str::to_owned))
            .unwrap_or_default();
        last_error = match (status.as_u16(), code.as_str()) {
            (401, _) => "收件箱：令牌无效或已过期".into(),
            (403, _) => "收件箱：令牌没有投递权限".into(),
            (409, "destination_changed") => "收件箱的数据已更换，请重新发送".into(),
            (409, "ingest_in_progress") => "收件箱正在处理这次投递".into(),
            (413 | 422, _) => "内容超出收件箱的限制".into(),
            (code, _) => format!("收件箱：HTTP {code}"),
        };
        let retry = status.is_server_error() || code == "ingest_in_progress";
        if !retry {
            break;
        }
    }
    Err(last_error)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn finds_links_in_mixed_text() {
        let text = "看这个 https://example.com/a?b=1#c，还有（https://docs.rs/serde）。\nhttp://Example.org/x. https://example.com/a?b=1#c ftp://nope";
        assert_eq!(
            extract_links(text),
            [
                "https://example.com/a?b=1#c",
                "https://docs.rs/serde",
                "http://Example.org/x"
            ]
        );
        assert!(extract_links("没有链接 https:// 也不算").is_empty());
    }

    #[test]
    fn builds_a_manifest_matching_the_ingest_contract() {
        let id = "88bdde6b-4cd7-43dc-a24b-3ef67f7fd6e8";
        let m = manifest(id, Some("  稍后看  "), " 读一下 https://example.com \n").unwrap();
        assert_eq!(m["schema_version"], 1);
        assert_eq!(m["client_submission_id"], id);
        assert_eq!(m["title"], "稍后看");
        assert_eq!(m["body_text"], "读一下 https://example.com");
        assert_eq!(m["links"][0]["url"], "https://example.com");
        assert_eq!(m["source"]["kind"], "script");
        assert!(manifest(id, Some(""), "x").unwrap()["title"].is_null());
        assert!(manifest(id, None, "   ").is_err());
        assert!(manifest(id, Some(&"长".repeat(300)), "x").is_err());
    }
}
