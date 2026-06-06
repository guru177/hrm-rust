use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workflow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
    pub is_active: bool,
    pub execution_count: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

impl Workflow {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let actions_str: Option<String> = row.get("actions")?;
        let actions = actions_str.and_then(|s| serde_json::from_str(&s).ok());
        let trigger_conditions_str: Option<String> = row.get("trigger_conditions")?;
        let trigger_conditions = trigger_conditions_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            trigger_type: row.get("trigger_type")?,
            trigger_conditions,
            actions,
            is_active: row.get::<_, Option<bool>>("is_active")?.unwrap_or(false),
            execution_count: row.get("execution_count")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}
