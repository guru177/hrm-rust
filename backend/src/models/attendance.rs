use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Attendance {
    pub id: i64,
    pub user_id: i64,
    pub date: String,
    pub clock_in: Option<String>,
    pub clock_out: Option<String>,
    pub duration_minutes: Option<i64>,
    pub is_late: bool,
    pub is_early_exit: bool,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub clock_in_location: Option<String>,
    pub clock_in_face_match_score: Option<f64>,
    pub clock_in_face_verified: bool,
    pub source: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeoLocation {
    pub lat: f64,
    pub lng: f64,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpLocation {
    pub ip: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationPayload {
    pub geo: GeoLocation,
    pub ip: IpLocation,
}

#[derive(Debug, Deserialize)]
pub struct ClockInRequest {
    pub face_verified: Option<bool>,
    pub face_match_score: Option<f64>,
    pub location: Option<LocationPayload>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub photo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AttendanceListQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl Attendance {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            date: row.get("date")?,
            clock_in: row.get("clock_in")?,
            clock_out: row.get("clock_out")?,
            duration_minutes: row.get("duration_minutes").ok(),
            is_late: row.get::<_, i64>("is_late").unwrap_or(0) != 0,
            is_early_exit: row.get::<_, i64>("is_early_exit").unwrap_or(0) != 0,
            notes: row.get("notes").ok(),
            status: row.get("status").ok(),
            clock_in_location: row.get("clock_in_location").ok(),
            clock_in_face_match_score: row.get("clock_in_face_match_score").ok(),
            clock_in_face_verified: row.get::<_, i64>("clock_in_face_verified").unwrap_or(0) != 0,
            source: row.get("source").ok(),
            created_at: row.get("created_at").ok(),
            updated_at: row.get("updated_at").ok(),
        })
    }
}
