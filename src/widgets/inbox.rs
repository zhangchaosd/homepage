//! Cross-device inbox (inboxd): token status and, when the token may read,
//! the most recent items for the card; quick sends with the script token; and,
//! with a read-write token from inboxd 0.4.0 on, single items: detail,
//! attachments, title, pin, trash and undo.
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
struct ItemPage {
    items: Vec<ItemSummary>,
}

#[derive(Deserialize)]
struct ItemSummary {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    summary: String,
    kind: String,
    created_at: String,
    #[serde(default)]
    pinned: bool,
    expires_at: Option<String>,
    #[serde(default)]
    attachment_count: u32,
    source: Option<Source>,
}

#[derive(Deserialize)]
struct Source {
    label: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemView {
    id: String,
    title: String,
    summary: String,
    kind: String,
    created_at: String,
    pinned: bool,
    expires_at: Option<String>,
    attachments: u32,
    source: Option<String>,
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
    let has = |name: &str| me.scopes.iter().any(|scope| scope == name);
    let can_send = has("ingest:write");
    let can_read = has("items:read");
    let can_manage = can_read && has("items:write") && has("items:trash");
    // Recent items need a token that can read; a write-only token just sends.
    let recent = if can_read {
        let limit = config.limit.to_string();
        let page: ItemPage = get_json(
            client
                .get(format!("{}/api/v1/items", config.url))
                .bearer_auth(&config.token)
                .query(&[("state", "active"), ("limit", limit.as_str())]),
            "inbox",
        )
        .await?;
        Some(page.items.into_iter().map(view).collect::<Vec<_>>())
    } else {
        None
    };
    Ok(json!({
        "expiresAt": me.expires_at,
        "canSend": can_send,
        "canManage": can_manage,
        "recent": recent,
        "link": config.link,
    }))
}

fn view(item: ItemSummary) -> ItemView {
    ItemView {
        id: item.id,
        title: item.title,
        summary: item.summary,
        kind: item.kind,
        created_at: item.created_at,
        pinned: item.pinned,
        expires_at: item.expires_at,
        attachments: item.attachment_count,
        source: item.source.and_then(|source| source.label),
    }
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

// ---------------------------------------------------------------- one item

#[derive(Deserialize)]
struct Detail {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    body_text: String,
    #[serde(default)]
    links: Vec<Link>,
    kind: String,
    source: Option<Source>,
    created_at: String,
    #[serde(default)]
    pinned: bool,
    expires_at: Option<String>,
    version: i64,
    #[serde(default)]
    attachments: Vec<Attachment>,
}

#[derive(Deserialize, Serialize)]
struct Link {
    url: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Deserialize)]
struct Attachment {
    id: String,
    original_filename: String,
    #[serde(default)]
    detected_media_type: String,
    #[serde(default)]
    byte_length: u64,
    #[serde(default)]
    preview_status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailView {
    id: String,
    title: String,
    text: String,
    links: Vec<Link>,
    kind: String,
    source: Option<String>,
    created_at: String,
    pinned: bool,
    expires_at: Option<String>,
    version: i64,
    attachments: Vec<AttachmentView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttachmentView {
    id: String,
    name: String,
    media_type: String,
    size: u64,
    preview: bool,
}

impl From<Detail> for DetailView {
    fn from(item: Detail) -> Self {
        Self {
            id: item.id,
            title: item.title,
            text: item.body_text,
            links: item.links,
            kind: item.kind,
            source: item.source.and_then(|source| source.label),
            created_at: item.created_at,
            pinned: item.pinned,
            expires_at: item.expires_at,
            version: item.version,
            attachments: item
                .attachments
                .into_iter()
                .map(|file| AttachmentView {
                    id: file.id,
                    name: file.original_filename,
                    media_type: file.detected_media_type,
                    size: file.byte_length,
                    preview: file.preview_status == "ready",
                })
                .collect(),
        }
    }
}

/// Item and attachment ids are UUIDs; anything else never reaches inboxd.
pub fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok()
}

/// Sends one single-item request and turns inboxd's error codes into words.
async fn item_call(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let response = request
        .send()
        .await
        .map_err(|error| super::describe("收件箱", &error))?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let code = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|body| body["error"]["code"].as_str().map(str::to_owned))
        .unwrap_or_default();
    Err(match (status.as_u16(), code.as_str()) {
        (401, _) => "收件箱：令牌无效或已过期".into(),
        (403, _) => {
            "收件箱的令牌不能整理条目：请在收件箱「设置 → 令牌」新建一个读写令牌，换到主页配置里"
                .into()
        }
        (404 | 410, _) => "这一条已经不在收件箱里了".into(),
        (409 | 412, _) => "这一条刚在别处改过，请关掉重新打开".into(),
        (429, _) => "收件箱：请求太频繁，请稍后再试".into(),
        (code, _) => format!("收件箱：HTTP {code}"),
    })
}

async fn detail_of(response: reqwest::Response) -> Result<DetailView, String> {
    response
        .json::<Detail>()
        .await
        .map(DetailView::from)
        .map_err(|_| "收件箱：条目数据格式异常".into())
}

pub async fn item(
    client: &reqwest::Client,
    config: &Inbox,
    id: &str,
) -> Result<DetailView, String> {
    let response = item_call(
        client
            .get(format!("{}/api/v1/items/{id}", config.url))
            .bearer_auth(&config.token),
    )
    .await?;
    detail_of(response).await
}

/// Changes the title and/or pin of the item at `version`.
pub async fn update(
    client: &reqwest::Client,
    config: &Inbox,
    id: &str,
    version: i64,
    title: Option<&str>,
    pinned: Option<bool>,
) -> Result<DetailView, String> {
    let mut patch = serde_json::Map::new();
    if let Some(title) = title {
        let title = title.trim();
        if title.chars().count() > TITLE_MAX || title.chars().any(char::is_control) {
            return Err(format!("标题不能超过 {TITLE_MAX} 个字符"));
        }
        patch.insert("title".into(), json!(title));
    }
    if let Some(pinned) = pinned {
        patch.insert("pinned".into(), json!(pinned));
    }
    let response = item_call(
        client
            .patch(format!("{}/api/v1/items/{id}", config.url))
            .bearer_auth(&config.token)
            .header("If-Match", format!("\"{version}\""))
            .json(&patch),
    )
    .await?;
    detail_of(response).await
}

/// Moves the item to the trash (`restore: false`) or back. Returns the new
/// version, which an undo needs.
pub async fn trash(
    client: &reqwest::Client,
    config: &Inbox,
    id: &str,
    version: i64,
    restore: bool,
) -> Result<i64, String> {
    let action = if restore { "restore" } else { "trash" };
    let response = item_call(
        client
            .post(format!("{}/api/v1/items/{id}/{action}", config.url))
            .bearer_auth(&config.token)
            .header("If-Match", format!("\"{version}\""))
            .json(&json!({})),
    )
    .await?;
    response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|item| item["version"].as_i64())
        .ok_or_else(|| "收件箱：条目数据格式异常".into())
}

/// Opens an attachment (or its preview image) for streaming to the browser.
pub async fn attachment(
    client: &reqwest::Client,
    config: &Inbox,
    id: &str,
    preview: bool,
) -> Result<reqwest::Response, String> {
    let which = if preview { "preview" } else { "content" };
    item_call(
        client
            .get(format!("{}/api/v1/attachments/{id}/{which}", config.url))
            .bearer_auth(&config.token)
            // Large files take longer than the client's default timeout.
            .timeout(Duration::from_secs(600)),
    )
    .await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn maps_item_detail() {
        let detail: Detail = serde_json::from_value(json!({
            "id": "0f8fad5b-d9cb-469f-a165-70867728950e", "title": "", "body_text": "正文",
            "links": [{"url": "https://example.com"}], "kind": "mixed",
            "source": {"kind": "browser", "label": "iPhone"}, "client_captured_at": null,
            "created_at": "2026-10-17T10:00:00Z", "updated_at": "2026-10-17T10:00:00Z",
            "version": 3, "pinned": true, "retention_mode": "expires", "expires_at": null,
            "state": "active", "trashed_at": null, "purge_after": null,
            "attachments": [{"id": "a", "item_id": "x", "ordinal": 0, "original_filename": "a.jpg",
                "declared_media_type": "image/jpeg", "detected_media_type": "image/jpeg",
                "source_uti": null, "byte_length": 12, "sha256": "s", "preview_status": "ready",
                "preview_byte_length": 4}]
        }))
        .unwrap();
        let view = serde_json::to_value(DetailView::from(detail)).unwrap();
        assert_eq!(view["text"], "正文");
        assert_eq!(view["source"], "iPhone");
        assert_eq!(view["attachments"][0]["preview"], true);
        assert_eq!(view["attachments"][0]["name"], "a.jpg");
        assert!(valid_id("0f8fad5b-d9cb-469f-a165-70867728950e"));
        assert!(!valid_id("../settings"));
    }

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
