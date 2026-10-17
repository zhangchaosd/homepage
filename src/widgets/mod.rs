//! Background fetchers. Each widget refreshes on its own interval into an
//! in-memory slot; the dashboard endpoint only reads slots, so the page never
//! waits on an upstream service. A failed refresh keeps the last good data and
//! records the error next to it.

mod bookmarkd;
mod devices;
mod forgejo;
mod github;
mod health;
mod homeassistant;
pub mod vikunja;

use std::{collections::HashMap, sync::Arc, time::Duration};

use serde::Serialize;
use serde_json::Value;
use tokio::sync::{Notify, RwLock};

use crate::config::Config;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Widget {
    Health,
    Devices,
    Vikunja,
    Forgejo,
    Github,
    Bookmarkd,
    HomeAssistant,
}

impl Widget {
    pub fn key(self) -> &'static str {
        match self {
            Self::Health => "health",
            Self::Devices => "devices",
            Self::Vikunja => "vikunja",
            Self::Forgejo => "forgejo",
            Self::Github => "github",
            Self::Bookmarkd => "bookmarkd",
            Self::HomeAssistant => "homeassistant",
        }
    }

    pub fn parse(key: &str) -> Option<Self> {
        [
            Self::Health,
            Self::Devices,
            Self::Vikunja,
            Self::Forgejo,
            Self::Github,
            Self::Bookmarkd,
            Self::HomeAssistant,
        ]
        .into_iter()
        .find(|widget| widget.key() == key)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub data: Option<Value>,
    pub error: Option<String>,
    /// Unix seconds of the last successful refresh.
    pub updated_at: Option<i64>,
    /// Unix seconds of the last attempt.
    pub checked_at: Option<i64>,
}

pub type FetchResult = Result<Value, String>;

#[derive(Clone)]
pub struct Hub {
    config: Arc<Config>,
    pub client: reqwest::Client,
    probe: reqwest::Client,
    slots: Arc<RwLock<HashMap<Widget, Slot>>>,
    wakers: Arc<HashMap<Widget, Arc<Notify>>>,
}

impl Hub {
    pub fn new(config: Arc<Config>) -> Result<Self, reqwest::Error> {
        let user_agent = concat!("homepage/", env!("CARGO_PKG_VERSION"));
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(4))
            .build()?;
        // Status probes only measure reachability, so LAN services with
        // self-signed certificates still count as up.
        let probe = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .danger_accept_invalid_certs(true)
            .build()?;
        let wakers = Self::enabled(&config)
            .into_iter()
            .map(|(widget, _)| (widget, Arc::new(Notify::new())))
            .collect();
        Ok(Self {
            config,
            client,
            probe,
            slots: Arc::new(RwLock::new(HashMap::new())),
            wakers: Arc::new(wakers),
        })
    }

    /// Widgets that are configured, with their refresh interval.
    fn enabled(config: &Config) -> Vec<(Widget, Duration)> {
        let mut widgets = Vec::new();
        if config
            .services
            .iter()
            .any(|service| health::target(service).is_some())
        {
            widgets.push((Widget::Health, Duration::from_secs(60)));
        }
        if !config.devices.is_empty() {
            widgets.push((Widget::Devices, Duration::from_secs(60)));
        }
        if let Some(section) = &config.vikunja {
            widgets.push((Widget::Vikunja, section.refresh));
        }
        if let Some(section) = &config.forgejo {
            widgets.push((Widget::Forgejo, section.refresh));
        }
        if let Some(section) = &config.github {
            widgets.push((Widget::Github, section.refresh));
        }
        if let Some(section) = &config.bookmarkd {
            widgets.push((Widget::Bookmarkd, section.refresh));
        }
        if let Some(section) = &config.homeassistant {
            widgets.push((Widget::HomeAssistant, section.refresh));
        }
        widgets
    }

    pub fn start(&self) {
        for (widget, interval) in Self::enabled(&self.config) {
            let hub = self.clone();
            tokio::spawn(async move { hub.run(widget, interval).await });
        }
    }

    async fn run(self, widget: Widget, interval: Duration) {
        let Some(waker) = self.wakers.get(&widget).cloned() else {
            return;
        };
        loop {
            self.refresh(widget).await;
            tokio::select! {
                () = tokio::time::sleep(interval) => {}
                () = waker.notified() => {}
            }
        }
    }

    async fn refresh(&self, widget: Widget) {
        let config = &self.config;
        let result = match widget {
            Widget::Health => health::fetch(&self.probe, &config.services).await,
            Widget::Devices => devices::fetch(&self.client, &config.devices).await,
            Widget::Vikunja => match &config.vikunja {
                Some(section) => vikunja::fetch(&self.client, section).await,
                None => return,
            },
            Widget::Forgejo => match &config.forgejo {
                Some(section) => forgejo::fetch(&self.client, section).await,
                None => return,
            },
            Widget::Github => match &config.github {
                Some(section) => github::fetch(&self.client, section).await,
                None => return,
            },
            Widget::Bookmarkd => match &config.bookmarkd {
                Some(section) => bookmarkd::fetch(&self.client, section).await,
                None => return,
            },
            Widget::HomeAssistant => match &config.homeassistant {
                Some(section) => homeassistant::fetch(&self.client, section).await,
                None => return,
            },
        };
        let now = chrono::Utc::now().timestamp();
        let mut slots = self.slots.write().await;
        let slot = slots.entry(widget).or_default();
        slot.checked_at = Some(now);
        match result {
            Ok(data) => {
                slot.data = Some(data);
                slot.error = None;
                slot.updated_at = Some(now);
            }
            Err(error) => {
                tracing::warn!(widget = widget.key(), %error, "refresh failed");
                slot.error = Some(error);
            }
        }
    }

    /// Asks a widget to refresh now instead of waiting for its interval.
    pub fn wake(&self, widget: Widget) -> bool {
        match self.wakers.get(&widget) {
            Some(waker) => {
                waker.notify_one();
                true
            }
            None => false,
        }
    }

    pub async fn snapshot(&self) -> HashMap<&'static str, Slot> {
        let slots = self.slots.read().await;
        self.wakers
            .keys()
            .map(|widget| (widget.key(), slots.get(widget).cloned().unwrap_or_default()))
            .collect()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Sends a request and decodes JSON, turning every failure into a short message
/// that is safe to show on the dashboard (no tokens, no response bodies).
pub async fn get_json<T: serde::de::DeserializeOwned>(
    request: reqwest::RequestBuilder,
    what: &str,
) -> Result<T, String> {
    let response = request
        .send()
        .await
        .map_err(|error| describe(what, &error))?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => format!("{what}：令牌被拒绝（HTTP {}）", status.as_u16()),
            _ => format!("{what}：HTTP {}", status.as_u16()),
        });
    }
    response.json::<T>().await.map_err(|error| {
        tracing::debug!(%error, what, "undecodable response");
        format!("{what}：返回数据格式异常")
    })
}

pub fn describe(what: &str, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        format!("{what}：请求超时")
    } else if error.is_connect() {
        format!("{what}：无法连接")
    } else {
        format!("{what}：请求失败")
    }
}
