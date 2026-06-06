use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Payslip {
    pub id: i64,
    pub user_id: i64,
    pub month: i32,
    pub year: i32,
    pub gross_salary: f64,
    pub total_deductions: f64,
    pub net_salary: f64,
    pub status: String,
    pub generated_at: Option<String>,
    pub components: Option<String>,
    pub adjustments: Option<String>,
    pub download_token: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Payslip {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            month: row.get("month")?,
            year: row.get("year")?,
            gross_salary: row.get("gross_salary")?,
            total_deductions: row.get("total_deductions")?,
            net_salary: row.get("net_salary")?,
            status: row.get("status")?,
            generated_at: row.get("generated_at").ok().flatten(),
            components: row.get("components")?,
            adjustments: row.get("adjustments").ok().flatten(),
            download_token: row.get("download_token").ok().flatten(),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}
