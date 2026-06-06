use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: Option<String>,
    pub assigned_to: Option<i64>,
    pub project_id: Option<i64>,
    pub due_date: Option<String>,
    pub development_type: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assigned_to: Option<i64>,
    pub project_id: Option<i64>,
    pub due_date: Option<String>,
    pub development_type: Option<String>,
}

/// Accepts UI values like "unassigned" / "none" for optional IDs.
#[derive(Debug, Deserialize)]
pub struct TaskStoreBody {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<serde_json::Value>,
    #[serde(default)]
    pub project_id: Option<serde_json::Value>,
    pub due_date: Option<String>,
    #[serde(alias = "type")]
    pub development_type: Option<String>,
}

pub fn parse_optional_task_id(value: &Option<serde_json::Value>) -> Option<i64> {
    match value {
        None => None,
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        Some(serde_json::Value::String(s)) => {
            if s.is_empty() || s == "none" || s == "unassigned" {
                None
            } else {
                s.parse().ok()
            }
        }
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskStatusRequest {
    pub status: String,
}

impl Task {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            status: row.get("status")?,
            priority: row.get("priority")?,
            assigned_to: row.get("assigned_to")?,
            project_id: row.get("project_id")?,
            due_date: row.get("due_date")?,
            development_type: row.get::<_, Option<String>>("type").ok().flatten(),
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}
