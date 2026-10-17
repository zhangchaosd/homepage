//! Reachability of each service: any HTTP answer below 500 counts as up.

use std::time::Instant;

use futures_util::future::join_all;
use serde::Serialize;
use serde_json::Value;

use super::FetchResult;
use crate::config::Service;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    up: bool,
    status: Option<u16>,
    latency_ms: Option<u64>,
    error: Option<String>,
}

pub fn target(service: &Service) -> Option<&str> {
    let url = service.check.as_deref().unwrap_or(&service.url).trim();
    (!url.is_empty()).then_some(url)
}

pub async fn fetch(client: &reqwest::Client, services: &[Service]) -> FetchResult {
    let checks = services.iter().filter_map(|service| {
        let url = target(service)?;
        Some(async move { (service.name.clone(), probe(client, url).await) })
    });
    let results = join_all(checks).await;
    let map: serde_json::Map<String, Value> = results
        .into_iter()
        .filter_map(|(name, status)| Some((name, serde_json::to_value(status).ok()?)))
        .collect();
    Ok(Value::Object(map))
}

async fn probe(client: &reqwest::Client, url: &str) -> Status {
    let started = Instant::now();
    match client.get(url).send().await {
        Ok(response) => {
            let code = response.status().as_u16();
            Status {
                up: code < 500,
                status: Some(code),
                latency_ms: Some(started.elapsed().as_millis() as u64),
                error: None,
            }
        }
        Err(error) => Status {
            up: false,
            status: None,
            latency_ms: None,
            error: Some(super::describe("probe", &error).replace("probe：", "")),
        },
    }
}
