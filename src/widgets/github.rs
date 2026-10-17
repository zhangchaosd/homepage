//! Public GitHub profile, recently pushed repositories and, with a token, the
//! contribution calendar. The avatar is inlined as a data URI so the browser
//! never talks to GitHub and the CSP can stay `img-src 'self' data:`.

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{FetchResult, get_json};
use crate::config::Github;

const API: &str = "https://api.github.com";

#[derive(Deserialize, Serialize)]
struct User {
    login: String,
    name: Option<String>,
    bio: Option<String>,
    html_url: String,
    avatar_url: String,
    #[serde(default)]
    public_repos: i64,
    #[serde(default)]
    followers: i64,
    #[serde(default)]
    following: i64,
}

#[derive(Deserialize)]
struct Repo {
    name: String,
    description: Option<String>,
    html_url: String,
    language: Option<String>,
    #[serde(default)]
    stargazers_count: i64,
    #[serde(default)]
    forks_count: i64,
    pushed_at: Option<String>,
    #[serde(default)]
    fork: bool,
    #[serde(default)]
    archived: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoView {
    name: String,
    description: Option<String>,
    url: String,
    language: Option<String>,
    stars: i64,
    forks: i64,
    pushed_at: Option<String>,
}

pub async fn fetch(client: &reqwest::Client, config: &Github) -> FetchResult {
    let request = |url: String| {
        let request = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        match &config.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    };
    let user: User = get_json(
        request(format!("{API}/users/{}", config.username)),
        "github",
    )
    .await?;
    let repos: Vec<Repo> = get_json(
        request(format!("{API}/users/{}/repos", config.username)).query(&[
            ("sort", "pushed"),
            ("per_page", "30"),
            ("type", "owner"),
        ]),
        "github repos",
    )
    .await?;
    let repos: Vec<RepoView> = repos
        .into_iter()
        .filter(|repo| !repo.fork && !repo.archived)
        .take(config.limit)
        .map(|repo| RepoView {
            name: repo.name,
            description: repo.description.filter(|text| !text.trim().is_empty()),
            url: repo.html_url,
            language: repo.language,
            stars: repo.stargazers_count,
            forks: repo.forks_count,
            pushed_at: repo.pushed_at,
        })
        .collect();

    let avatar = inline_avatar(client, &user.avatar_url).await;
    let contributions = match &config.token {
        Some(token) => contributions(client, token, &config.username).await.ok(),
        None => None,
    };
    Ok(serde_json::json!({
        "user": {
            "login": user.login,
            "name": user.name,
            "bio": user.bio,
            "url": user.html_url,
            "avatar": avatar,
            "publicRepos": user.public_repos,
            "followers": user.followers,
            "following": user.following,
        },
        "repos": repos,
        "contributions": contributions,
    }))
}

async fn inline_avatar(client: &reqwest::Client, url: &str) -> Option<String> {
    let separator = if url.contains('?') { '&' } else { '?' };
    let response = client
        .get(format!("{url}{separator}s=160"))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mime = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.starts_with("image/"))
        .unwrap_or("image/png")
        .to_owned();
    let bytes = response.bytes().await.ok()?;
    (bytes.len() <= 256 * 1024).then(|| format!("data:{mime};base64,{}", STANDARD.encode(&bytes)))
}

async fn contributions(
    client: &reqwest::Client,
    token: &str,
    login: &str,
) -> Result<Value, String> {
    let query = "query($login:String!){user(login:$login){contributionsCollection{contributionCalendar{totalContributions weeks{contributionDays{date contributionCount}}}}}}";
    let body: Value = get_json(
        client
            .post(format!("{API}/graphql"))
            .bearer_auth(token)
            .json(&serde_json::json!({ "query": query, "variables": { "login": login } })),
        "github graphql",
    )
    .await?;
    let calendar = &body["data"]["user"]["contributionsCollection"]["contributionCalendar"];
    let days: Vec<Value> = calendar["weeks"]
        .as_array()
        .ok_or("github graphql：没有贡献数据")?
        .iter()
        .flat_map(|week| {
            week["contributionDays"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .map(|day| serde_json::json!([day["date"], day["contributionCount"]]))
        .collect();
    Ok(serde_json::json!({ "total": calendar["totalContributions"], "days": days }))
}
