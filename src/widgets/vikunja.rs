//! Open Vikunja tasks that are overdue, due today, or due within
//! `upcoming_days`. The browser groups them in its own time zone.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{FetchResult, get_json};
use crate::config::Vikunja;

#[derive(Deserialize)]
struct Task {
    id: i64,
    title: String,
    #[serde(default)]
    done: bool,
    due_date: Option<String>,
    #[serde(default)]
    priority: i64,
    #[serde(default)]
    project_id: i64,
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    labels: Option<Vec<Label>>,
}

#[derive(Deserialize, Serialize)]
struct Label {
    title: String,
    #[serde(default)]
    hex_color: String,
}

#[derive(Deserialize)]
struct Project {
    id: i64,
    title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskView {
    id: i64,
    title: String,
    /// RFC 3339, absent when the task has no due date.
    due: Option<String>,
    priority: i64,
    project: Option<String>,
    identifier: String,
    labels: Vec<Label>,
    url: String,
}

/// Vikunja serializes "no due date" as the zero time.
fn real_date(value: Option<String>) -> Option<String> {
    value.filter(|date| !date.starts_with("0001-"))
}

pub async fn fetch(client: &reqwest::Client, config: &Vikunja) -> FetchResult {
    let horizon = config.upcoming_days + 1;
    let filter = format!("done = false && due_date < now/d+{horizon}d");
    let query = [
        ("filter", filter.as_str()),
        ("filter_include_nulls", "false"),
        ("sort_by", "due_date"),
        ("order_by", "asc"),
        ("per_page", "100"),
    ];
    let list = |path: &str| {
        get_json::<Vec<Task>>(
            client
                .get(format!("{}/api/v1/{path}", config.url))
                .bearer_auth(&config.token)
                .query(&query),
            "vikunja",
        )
    };
    // Older releases list across projects at /tasks/all, newer ones at /tasks.
    let tasks = match list("tasks/all").await {
        Err(error) if error.ends_with("HTTP 404") => list("tasks").await?,
        result => result?,
    };
    // Project names are nice to have; the list still renders without them.
    let projects: HashMap<i64, String> = get_json::<Vec<Project>>(
        client
            .get(format!("{}/api/v1/projects", config.url))
            .bearer_auth(&config.token)
            .query(&[("per_page", "200")]),
        "vikunja projects",
    )
    .await
    .map(|projects| {
        projects
            .into_iter()
            .map(|project| (project.id, project.title))
            .collect()
    })
    .unwrap_or_default();

    let views: Vec<TaskView> = tasks
        .into_iter()
        .filter(|task| !task.done)
        .map(|task| TaskView {
            url: format!("{}/tasks/{}", config.link, task.id),
            id: task.id,
            title: task.title,
            due: real_date(task.due_date),
            priority: task.priority,
            project: projects.get(&task.project_id).cloned(),
            identifier: task.identifier,
            labels: task.labels.unwrap_or_default(),
        })
        .collect();
    Ok(serde_json::json!({ "tasks": views, "upcomingDays": config.upcoming_days }))
}

/// Marks a task done. Vikunja's update replaces the whole task, so the current
/// task is read first and sent back with only `done` changed.
pub async fn complete(client: &reqwest::Client, config: &Vikunja, id: i64) -> Result<(), String> {
    let url = format!("{}/api/v1/tasks/{id}", config.url);
    let mut task: Value = get_json(client.get(&url).bearer_auth(&config.token), "vikunja").await?;
    let Some(object) = task.as_object_mut() else {
        return Err("vikunja：任务数据格式异常".into());
    };
    object.insert("done".into(), Value::Bool(true));
    let _: Value = get_json(
        client.post(&url).bearer_auth(&config.token).json(&task),
        "vikunja",
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_time_means_no_due_date() {
        assert_eq!(real_date(Some("0001-01-01T00:00:00Z".into())), None);
        assert!(real_date(Some("2026-10-17T18:00:00+08:00".into())).is_some());
    }
}
