//! Current state of the configured Home Assistant entities.

use futures_util::future::join_all;
use serde::{Deserialize, Serialize};

use super::{FetchResult, get_json};
use crate::config::{HaEntity, HomeAssistant};

#[derive(Deserialize)]
struct State {
    state: String,
    #[serde(default)]
    attributes: serde_json::Map<String, serde_json::Value>,
    last_changed: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityView {
    id: String,
    name: String,
    state: Option<String>,
    unit: Option<String>,
    device_class: Option<String>,
    icon: Option<String>,
    last_changed: Option<String>,
    error: Option<String>,
}

pub async fn fetch(client: &reqwest::Client, config: &HomeAssistant) -> FetchResult {
    let views = join_all(
        config
            .entities
            .iter()
            .map(|entity| query(client, config, entity)),
    )
    .await;
    // One failed entity is shown inline; all failing means HA itself is down.
    if !views.is_empty() && views.iter().all(|view| view.error.is_some()) {
        return Err(views[0]
            .error
            .clone()
            .unwrap_or_else(|| "homeassistant：请求失败".into()));
    }
    Ok(serde_json::json!({ "entities": views, "link": config.link }))
}

async fn query(client: &reqwest::Client, config: &HomeAssistant, entity: &HaEntity) -> EntityView {
    let result: Result<State, String> = get_json(
        client
            .get(format!("{}/api/states/{}", config.url, entity.id))
            .bearer_auth(&config.token),
        "homeassistant",
    )
    .await;
    let attribute = |state: &State, key: &str| {
        state
            .attributes
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    };
    match result {
        Ok(state) => EntityView {
            id: entity.id.clone(),
            name: entity
                .name
                .clone()
                .or_else(|| attribute(&state, "friendly_name"))
                .unwrap_or_else(|| entity.id.clone()),
            unit: attribute(&state, "unit_of_measurement"),
            device_class: attribute(&state, "device_class"),
            icon: entity.icon.clone(),
            last_changed: state.last_changed.clone(),
            state: Some(state.state),
            error: None,
        },
        Err(error) => EntityView {
            id: entity.id.clone(),
            name: entity.name.clone().unwrap_or_else(|| entity.id.clone()),
            state: None,
            unit: None,
            device_class: None,
            icon: entity.icon.clone(),
            last_changed: None,
            error: Some(error),
        },
    }
}
