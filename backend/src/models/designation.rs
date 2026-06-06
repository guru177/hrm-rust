use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Designation {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub level: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDesignationRequest {
    pub name: String,
    pub description: Option<String>,
    pub level: Option<i64>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

impl Designation {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            slug: row.get("slug")?,
            description: row.get("description")?,
            level: row.get("level").ok().flatten(),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}
