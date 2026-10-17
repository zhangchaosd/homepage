//! Recent commits from the user's Forgejo activity feed, plus the
//! contribution heatmap.

use serde::{Deserialize, Serialize};

use super::{FetchResult, get_json};
use crate::config::Forgejo;

#[derive(Deserialize)]
struct Activity {
    op_type: String,
    #[serde(default)]
    ref_name: String,
    #[serde(default)]
    content: String,
    repo: Option<Repo>,
    created: Option<String>,
}

#[derive(Deserialize)]
struct Repo {
    full_name: String,
}

/// The JSON stored in a `commit_repo` activity's `content`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PushCommits {
    #[serde(default)]
    commits: Vec<PushCommit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PushCommit {
    sha1: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    author_name: String,
    timestamp: Option<String>,
}

#[derive(Deserialize)]
struct HeatmapDay {
    timestamp: i64,
    contributions: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommitView {
    sha: String,
    title: String,
    repo: String,
    branch: String,
    author: String,
    time: Option<String>,
    url: String,
    repo_url: String,
}

pub async fn fetch(client: &reqwest::Client, config: &Forgejo) -> FetchResult {
    let auth = |request: reqwest::RequestBuilder| match &config.token {
        Some(token) => request.header("Authorization", format!("token {token}")),
        None => request,
    };
    let activities: Vec<Activity> = get_json(
        auth(client.get(format!(
            "{}/api/v1/users/{}/activities/feeds",
            config.url, config.username
        )))
        .query(&[("only-performed-by", "true"), ("limit", "50")]),
        "forgejo",
    )
    .await?;
    let commits = flatten(activities, &config.link, config.limit);

    let heatmap: Vec<[i64; 2]> = get_json::<Vec<HeatmapDay>>(
        auth(client.get(format!(
            "{}/api/v1/users/{}/heatmap",
            config.url, config.username
        ))),
        "forgejo heatmap",
    )
    .await
    .map(|days| {
        days.into_iter()
            .map(|day| [day.timestamp, day.contributions])
            .collect()
    })
    .unwrap_or_default();

    Ok(serde_json::json!({
        "commits": commits,
        "heatmap": heatmap,
        "profileUrl": format!("{}/{}", config.link, config.username),
    }))
}

fn flatten(activities: Vec<Activity>, link: &str, limit: usize) -> Vec<CommitView> {
    let mut commits = Vec::new();
    for activity in activities {
        if activity.op_type != "commit_repo" {
            continue;
        }
        let Some(repo) = activity.repo else { continue };
        let Ok(push) = serde_json::from_str::<PushCommits>(&activity.content) else {
            continue;
        };
        let branch = activity
            .ref_name
            .strip_prefix("refs/heads/")
            .unwrap_or(&activity.ref_name)
            .to_owned();
        for commit in push.commits {
            commits.push(CommitView {
                url: format!("{link}/{}/commit/{}", repo.full_name, commit.sha1),
                repo_url: format!("{link}/{}", repo.full_name),
                title: commit
                    .message
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_owned(),
                sha: commit.sha1,
                repo: repo.full_name.clone(),
                branch: branch.clone(),
                author: commit.author_name,
                time: commit.timestamp.or_else(|| activity.created.clone()),
            });
            if commits.len() >= limit {
                return commits;
            }
        }
    }
    commits
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn flattens_push_activities_newest_first() {
        let activities: Vec<Activity> = serde_json::from_value(serde_json::json!([
            {"op_type": "create_repo", "repo": {"full_name": "chao/x"}},
            {
                "op_type": "commit_repo",
                "ref_name": "refs/heads/main",
                "repo": {"full_name": "chao/homepage"},
                "created": "2026-10-17T10:00:00+08:00",
                "content": "{\"Commits\":[{\"Sha1\":\"abc123\",\"Message\":\"Add widgets\\n\\nbody\",\"AuthorName\":\"chao\",\"Timestamp\":\"2026-10-17T09:59:00+08:00\"},{\"Sha1\":\"def456\",\"Message\":\"Init\",\"AuthorName\":\"chao\"}],\"Len\":2}"
            }
        ]))
        .unwrap();
        let commits = flatten(activities, "https://git.example.com", 1);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].title, "Add widgets");
        assert_eq!(commits[0].branch, "main");
        assert_eq!(
            commits[0].url,
            "https://git.example.com/chao/homepage/commit/abc123"
        );
    }
}
