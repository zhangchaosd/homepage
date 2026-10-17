//! Global IPv6 addresses reported by ipv6who on each LAN device. The device's
//! name, OS and architecture also come from ipv6who (`system`), so the config
//! only lists URLs.

use futures_util::future::join_all;
use serde::{Deserialize, Serialize};

use super::FetchResult;
use crate::config::Device;

#[derive(Deserialize)]
struct Ipv6Response {
    address: Option<String>,
    interface: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    addresses: Vec<Address>,
    system: Option<System>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Address {
    address: String,
    prefix_len: Option<u8>,
    interface: Option<String>,
    #[serde(rename(deserialize = "type"))]
    kind: Option<String>,
    #[serde(default)]
    deprecated: bool,
}

/// ipv6who's host description; every field may be missing on older versions.
#[derive(Default, Deserialize)]
struct System {
    hostname: Option<String>,
    /// `linux`, `macos` or `windows`.
    os: Option<String>,
    /// Readable name such as "Fedora Linux 44 (Cloud Edition)" or "macOS 27.0.1".
    name: Option<String>,
    arch: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStatus {
    /// Hostname without a local domain suffix, or the URL's host when unknown.
    name: String,
    url: String,
    os: Option<String>,
    os_name: Option<String>,
    arch: Option<String>,
    address: Option<String>,
    interface: Option<String>,
    address_type: Option<String>,
    addresses: Vec<Address>,
    error: Option<String>,
}

pub async fn fetch(client: &reqwest::Client, devices: &[Device]) -> FetchResult {
    let statuses = join_all(devices.iter().map(|device| query(client, device))).await;
    serde_json::to_value(statuses).map_err(|error| error.to_string())
}

/// "Mac_mini.lan" → "Mac_mini"; addresses and plain names stay as they are.
fn short_hostname(hostname: &str) -> String {
    let hostname = hostname.trim().trim_end_matches('.');
    for suffix in [".lan", ".local", ".home", ".localdomain", ".internal"] {
        if let Some(short) = hostname.strip_suffix(suffix)
            && !short.is_empty()
        {
            return short.to_owned();
        }
    }
    hostname.to_owned()
}

fn url_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned())
}

async fn query(client: &reqwest::Client, device: &Device) -> DeviceStatus {
    let mut status = DeviceStatus {
        name: url_host(&device.url),
        url: device.url.clone(),
        os: None,
        os_name: None,
        arch: None,
        address: None,
        interface: None,
        address_type: None,
        addresses: Vec::new(),
        error: None,
    };
    let mut request = client.get(format!("{}/v1/ipv6", device.url));
    if let Some(token) = &device.token {
        request = request.bearer_auth(token);
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            status.error = Some(super::describe("ipv6who", &error).replace("ipv6who：", ""));
            return status;
        }
    };
    let code = response.status().as_u16();
    match code {
        200 | 503 => {}
        401 => {
            status.error = Some("令牌被拒绝".into());
            return status;
        }
        code => {
            status.error = Some(format!("HTTP {code}"));
            return status;
        }
    }
    // 503 ("no global address") still carries the host description.
    let Ok(body) = response.json::<Ipv6Response>().await else {
        status.error = Some(if code == 503 {
            "没有全局 IPv6 地址".into()
        } else {
            "返回数据格式异常".into()
        });
        return status;
    };
    let system = body.system.unwrap_or_default();
    if let Some(hostname) = system
        .hostname
        .as_deref()
        .filter(|name| !name.trim().is_empty())
    {
        status.name = short_hostname(hostname);
    }
    status.os = system.os;
    status.os_name = system.name;
    status.arch = system.arch;
    if code == 503 {
        status.error = Some("没有全局 IPv6 地址".into());
        return status;
    }
    status.address = body.address;
    status.interface = body.interface;
    status.address_type = body.kind;
    status.addresses = body.addresses;
    status
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn hostnames_lose_local_domain_suffixes() {
        assert_eq!(
            short_hostname("Mac_mini_M5Pro_10G_LAN.lan"),
            "Mac_mini_M5Pro_10G_LAN"
        );
        assert_eq!(short_hostname("server1.local."), "server1");
        assert_eq!(short_hostname("Windows11onPVE"), "Windows11onPVE");
        assert_eq!(short_hostname(".lan"), ".lan");
        assert_eq!(url_host("http://192.168.0.117:8066"), "192.168.0.117");
    }

    #[test]
    fn reads_the_system_report() {
        let body: Ipv6Response = serde_json::from_value(serde_json::json!({
            "address": "2001:db8::10", "interface": "eth0", "type": "static", "addresses": [],
            "system": {"hostname": "Dev", "os": "linux", "name": "Fedora Linux 44 (Cloud Edition)", "version": "44", "arch": "x86_64"}
        }))
        .unwrap();
        let system = body.system.unwrap();
        assert_eq!(system.hostname.as_deref(), Some("Dev"));
        assert_eq!(system.os.as_deref(), Some("linux"));
        // Older ipv6who without `system` still parses.
        let old: Ipv6Response =
            serde_json::from_value(serde_json::json!({"address": null, "addresses": []})).unwrap();
        assert!(old.system.is_none());
    }
}
