//! Open Vikunja tasks that are overdue, due today, due within `upcoming_days`,
//! or have no due date at all. The browser groups them in its own time zone.

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

#[derive(Deserialize, Serialize, Clone)]
pub struct LabelOption {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub hex_color: String,
}

#[derive(Deserialize)]
struct Project {
    id: i64,
    title: String,
    #[serde(default)]
    is_archived: bool,
}

#[derive(Serialize)]
pub struct ProjectOption {
    id: i64,
    title: String,
}

/// What the add-task form offers: real projects (saved filters have ids <= 0),
/// every label, and the project Vikunja files new tasks under by default.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    projects: Vec<ProjectOption>,
    labels: Vec<LabelOption>,
    default_project: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub project_id: Option<i64>,
    /// RFC 3339; absent for a task without a due date.
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub labels: Vec<i64>,
    /// Labels to create first, by title; an existing title is reused.
    #[serde(default)]
    pub new_labels: Vec<String>,
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
        // Vikunja treats a missing due date as matching `due_date < …` when nulls are included.
        ("filter_include_nulls", "true"),
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
    // Vikunja 2.x lists across projects at /tasks and answers /tasks/all with
    // 400; releases before it only had /tasks/all.
    let tasks = match list("tasks").await {
        Err(error) if error.ends_with("HTTP 404") || error.ends_with("HTTP 405") => {
            list("tasks/all").await?
        }
        result => result?,
    };
    // Project names are nice to have; the list still renders without them.
    let projects: HashMap<i64, String> = projects(client, config)
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

async fn projects(client: &reqwest::Client, config: &Vikunja) -> Result<Vec<Project>, String> {
    get_json(
        client
            .get(format!("{}/api/v1/projects", config.url))
            .bearer_auth(&config.token)
            .query(&[("per_page", "200")]),
        "vikunja projects",
    )
    .await
}

async fn labels(client: &reqwest::Client, config: &Vikunja) -> Result<Vec<LabelOption>, String> {
    get_json(
        client
            .get(format!("{}/api/v1/labels", config.url))
            .bearer_auth(&config.token)
            .query(&[("per_page", "200")]),
        "vikunja labels",
    )
    .await
}

pub async fn options(client: &reqwest::Client, config: &Vikunja) -> Result<Options, String> {
    let (projects, labels, user) = tokio::join!(
        projects(client, config),
        labels(client, config),
        get_json::<Value>(
            client
                .get(format!("{}/api/v1/user", config.url))
                .bearer_auth(&config.token),
            "vikunja user",
        ),
    );
    let projects: Vec<ProjectOption> = projects?
        .into_iter()
        .filter(|project| project.id > 0 && !project.is_archived)
        .map(|project| ProjectOption {
            id: project.id,
            title: project.title,
        })
        .collect();
    let preferred = user
        .ok()
        .and_then(|user| user["settings"]["default_project_id"].as_i64())
        .filter(|id| projects.iter().any(|project| project.id == *id));
    Ok(Options {
        default_project: preferred.or(projects.first().map(|project| project.id)),
        projects,
        labels: labels?,
    })
}

/// A calm label colour picked from the title, so a label keeps its colour.
fn label_color(title: &str) -> &'static str {
    const PALETTE: [&str; 8] = [
        "c46a3f", "3a74b8", "4a9a6a", "b0477a", "c9a227", "6a4fa0", "2a9bb8", "8a6d3b",
    ];
    let hash = title.bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte.into())
    });
    PALETTE[hash as usize % PALETTE.len()]
}

impl NewTask {
    /// Checks what can be checked before talking to Vikunja.
    pub fn validate(&self) -> Result<(), String> {
        let title = self.title.trim();
        if title.is_empty() || title.chars().count() > 250 {
            return Err("标题不能为空，也不能超过 250 个字".into());
        }
        if !(0..=5).contains(&self.priority) {
            return Err("优先级只能是 0 到 5".into());
        }
        if self.new_labels.len() > 20 || self.labels.len() > 50 {
            return Err("标签太多了".into());
        }
        self.due().map(|_| ())
    }

    fn due(&self) -> Result<Option<String>, String> {
        match self
            .due
            .as_deref()
            .map(str::trim)
            .filter(|due| !due.is_empty())
        {
            Some(due) => chrono::DateTime::parse_from_rfc3339(due)
                .map(|due| Some(due.to_rfc3339()))
                .map_err(|_| "截止时间格式不正确".into()),
            None => Ok(None),
        }
    }
}

/// Creates a task: new labels first, then the task, then its labels.
pub async fn create(
    client: &reqwest::Client,
    config: &Vikunja,
    task: NewTask,
) -> Result<i64, String> {
    task.validate()?;
    let title = task.title.trim();
    let due = task.due()?;
    let project = match task.project_id {
        Some(id) => id,
        None => options(client, config)
            .await?
            .default_project
            .ok_or("vikunja：没有可用的项目")?,
    };

    let mut label_ids = task.labels;
    if !task.new_labels.is_empty() {
        let existing = labels(client, config).await?;
        for name in &task.new_labels {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 250 {
                continue;
            }
            let id = match existing
                .iter()
                .find(|label| label.title.eq_ignore_ascii_case(name))
            {
                Some(label) => label.id,
                None => {
                    let created: LabelOption = get_json(
                        client
                            .put(format!("{}/api/v1/labels", config.url))
                            .bearer_auth(&config.token)
                            .json(&serde_json::json!({ "title": name, "hex_color": label_color(name) })),
                        "vikunja labels",
                    )
                    .await?;
                    created.id
                }
            };
            label_ids.push(id);
        }
    }
    label_ids.sort_unstable();
    label_ids.dedup();

    let mut body = serde_json::json!({ "title": title, "priority": task.priority });
    if let Some(due) = due {
        body["due_date"] = Value::String(due);
    }
    let created: Value = get_json(
        client
            .put(format!("{}/api/v1/projects/{project}/tasks", config.url))
            .bearer_auth(&config.token)
            .json(&body),
        "vikunja",
    )
    .await?;
    let id = created["id"].as_i64().ok_or("vikunja：任务数据格式异常")?;
    if !label_ids.is_empty() {
        let labels: Vec<Value> = label_ids
            .iter()
            .map(|id| serde_json::json!({ "id": id }))
            .collect();
        // The task exists either way; a label failure is reported but not undone.
        let _: Value = get_json(
            client
                .post(format!("{}/api/v1/tasks/{id}/labels/bulk", config.url))
                .bearer_auth(&config.token)
                .json(&serde_json::json!({ "labels": labels })),
            "vikunja labels",
        )
        .await
        .map_err(|error| format!("任务已创建，但标签没有加上：{error}"))?;
    }
    Ok(id)
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

    fn task(title: &str, due: Option<&str>, priority: i64) -> NewTask {
        NewTask {
            title: title.into(),
            project_id: None,
            due: due.map(str::to_owned),
            priority,
            labels: vec![],
            new_labels: vec![],
        }
    }

    #[test]
    fn validates_new_tasks() {
        assert!(
            task("买猫粮", Some("2026-10-18T18:00:00+08:00"), 2)
                .validate()
                .is_ok()
        );
        assert!(task("买猫粮", None, 0).validate().is_ok());
        assert!(task("   ", None, 0).validate().is_err());
        assert!(task("买猫粮", Some("明天"), 0).validate().is_err());
        assert!(task("买猫粮", None, 9).validate().is_err());
        assert_eq!(label_color("家"), label_color("家"));
    }

    #[test]
    fn zero_time_means_no_due_date() {
        assert_eq!(real_date(Some("0001-01-01T00:00:00Z".into())), None);
        assert!(real_date(Some("2026-10-17T18:00:00+08:00".into())).is_some());
    }
}
