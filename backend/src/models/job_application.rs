use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobApplication {
    pub id: i64,
    pub career_id: Option<i64>,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub resume_url: Option<String>,
    pub cover_letter: Option<String>,
    pub status: String,
    pub tracking_number: Option<String>,
    pub dob: Option<String>,
    pub interview_date: Option<String>,
    pub interview_time: Option<String>,
    pub interview_center_id: Option<i64>,
    pub ats_score: Option<i64>,
    pub ats_feedback: Option<String>,
    pub parsed_skills: Option<String>,
    pub source: Option<String>,
    pub experience_years: Option<i64>,
    pub expected_salary: Option<String>,
    pub applied_position: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobApplicationRequest {
    pub career_id: Option<i64>,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub cover_letter: Option<String>,
    pub date_of_birth: Option<String>,
    pub applied_position: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

impl JobApplication {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            career_id: row.get("career_id")?,
            name: row.get("name")?,
            email: row.get("email")?,
            phone: row.get("phone").ok().flatten(),
            resume_url: row.get("resume").ok().flatten(),
            cover_letter: row.get("cover_letter").ok().flatten(),
            status: row.get("status")?,
            tracking_number: row.get("tracking_number").ok().flatten(),
            dob: row.get("dob").ok().flatten(),
            interview_date: row.get("interview_date").ok().flatten(),
            interview_time: row.get("interview_time").ok().flatten(),
            interview_center_id: row.get("interview_center_id").ok().flatten(),
            ats_score: row.get("ats_score").ok().flatten(),
            ats_feedback: row.get("ats_feedback").ok().flatten(),
            parsed_skills: row.get("parsed_skills").ok().flatten(),
            source: row.get("source").ok().flatten(),
            experience_years: row.get("experience_years").ok().flatten(),
            expected_salary: row.get("expected_salary").ok().flatten(),
            applied_position: row.get("applied_position").ok().flatten(),
            created_at: row.get("created_at").ok().flatten(),
            updated_at: row.get("updated_at").ok().flatten(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct JobApplicationQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub experience: Option<String>,
    pub career_id: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
