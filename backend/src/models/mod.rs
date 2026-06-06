pub mod user;
pub mod department;
pub mod designation;
pub mod role;
pub mod permission;
pub mod attendance;
pub mod leave_request;
pub mod holiday;
pub mod project;
pub mod task;
pub mod workflow;
pub mod payslip;
pub mod salary;
pub mod career;
pub mod job_application;
pub mod biometric;

// Re-export common response types
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub success: bool,
    #[serde(rename = "type")]
    pub response_type: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub success: bool,
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            response_type: "success".to_string(),
            data,
        }
    }
}

impl ApiError {
    pub fn new(message: &str) -> Self {
        Self {
            success: false,
            response_type: "error".to_string(),
            message: message.to_string(),
        }
    }
}
