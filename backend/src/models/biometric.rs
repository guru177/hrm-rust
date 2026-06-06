use serde::{Deserialize, Serialize};

/// A registered biometric device
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BiometricDevice {
    pub id: i64,
    pub serial_number: String,
    pub name: Option<String>,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub is_active: bool,
    pub last_heartbeat: Option<String>,
    pub firmware_version: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl BiometricDevice {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            serial_number: row.get("serial_number")?,
            name: row.get("name")?,
            model: row.get("model")?,
            ip_address: row.get("ip_address")?,
            location: row.get("location")?,
            is_active: row.get::<_, i64>("is_active").unwrap_or(1) == 1,
            last_heartbeat: row.get("last_heartbeat")?,
            firmware_version: row.get("firmware_version")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

/// A raw punch log from the biometric device
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BiometricPunch {
    pub id: i64,
    pub device_serial: String,
    pub device_pin: String,
    pub punch_time: String,
    pub punch_type: i64,
    pub verify_method: i64,
    pub user_id: Option<i64>,
    pub user_name: Option<String>,
    pub is_processed: bool,
    pub created_at: Option<String>,
}

impl BiometricPunch {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            device_serial: row.get("device_serial")?,
            device_pin: row.get("device_pin")?,
            punch_time: row.get("punch_time")?,
            punch_type: row.get("punch_type")?,
            verify_method: row.get("verify_method")?,
            user_id: row.get("user_id")?,
            user_name: None, // populated via JOIN
            is_processed: row.get::<_, i64>("is_processed").unwrap_or(0) == 1,
            created_at: row.get("created_at")?,
        })
    }
}

/// Mapping between device PIN and HRM user
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BiometricUserMap {
    pub id: i64,
    pub device_serial: String,
    pub device_pin: String,
    pub user_id: i64,
    pub user_name: Option<String>,
    pub created_at: Option<String>,
}

impl BiometricUserMap {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            device_serial: row.get("device_serial")?,
            device_pin: row.get("device_pin")?,
            user_id: row.get("user_id")?,
            user_name: None,
            created_at: row.get("created_at")?,
        })
    }
}

/// Request body for creating/updating user mapping
#[derive(Debug, Deserialize)]
pub struct UserMapRequest {
    pub device_serial: String,
    pub device_pin: String,
    pub user_id: i64,
}

/// Query parameters for iClock requests
#[derive(Debug, Deserialize)]
pub struct IClockQuery {
    #[serde(rename = "SN")]
    pub sn: Option<String>,
    pub table: Option<String>,
    #[serde(rename = "Stamp")]
    pub stamp: Option<String>,
}
