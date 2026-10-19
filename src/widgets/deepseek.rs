//! DeepSeek platform balance: whether the key can still be used and how much
//! credit is left, split into granted and topped-up.

use serde::{Deserialize, Serialize};

use super::{FetchResult, get_json};
use crate::config::Deepseek;

#[derive(Deserialize)]
struct Balance {
    #[serde(default)]
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct BalanceInfo {
    currency: String,
    /// The platform sends decimals as strings ("110.00").
    total_balance: String,
    #[serde(default)]
    granted_balance: String,
    #[serde(default)]
    topped_up_balance: String,
}

pub async fn fetch(client: &reqwest::Client, config: &Deepseek) -> FetchResult {
    let balance: Balance = get_json(
        client
            .get(format!("{}/user/balance", config.url))
            .bearer_auth(&config.token),
        "deepseek",
    )
    .await?;
    Ok(serde_json::json!({
        "available": balance.is_available,
        "balances": balance.balance_infos,
        "link": config.link,
    }))
}
