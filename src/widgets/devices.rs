//! Global IPv6 addresses reported by ipv6who on each LAN device.

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStatus {
    name: String,
    kind: String,
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

async fn query(client: &reqwest::Client, device: &Device) -> DeviceStatus {
    let mut status = DeviceStatus {
        name: device.name.clone(),
        kind: device.kind.clone(),
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
    match response.status().as_u16() {
        200 => {}
        503 => {
            status.error = Some("没有全局 IPv6 地址".into());
            return status;
        }
        401 => {
            status.error = Some("令牌被拒绝".into());
            return status;
        }
        code => {
            status.error = Some(format!("HTTP {code}"));
            return status;
        }
    }
    match response.json::<Ipv6Response>().await {
        Ok(body) => {
            status.address = body.address;
            status.interface = body.interface;
            status.address_type = body.kind;
            status.addresses = body.addresses;
        }
        Err(_) => status.error = Some("返回数据格式异常".into()),
    }
    status
}
