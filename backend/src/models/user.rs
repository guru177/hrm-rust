use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub photo: Option<String>,
    pub bio: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub department_id: Option<i64>,
    pub designation_id: Option<i64>,
    pub manager_id: Option<i64>,
    pub reporting_manager_id: Option<i64>,
    pub employee_id: Option<String>,
    pub join_date: Option<String>,
    pub date_of_joining: Option<String>,
    pub date_of_exit: Option<String>,
    pub work_location: Option<String>,
    pub employment_type: Option<String>,
    pub status: Option<String>,
    pub timezone: Option<String>,
    pub is_super_admin: bool,
    pub onboarded: bool,
    pub account_number: Option<String>,
    pub ifsc_code: Option<String>,
    pub bank_name: Option<String>,
    pub pan_number: Option<String>,
    pub esi_number: Option<String>,
    pub pf_number: Option<String>,
    pub aadhar_number: Option<String>,
    pub email_verified_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub photo: Option<String>,
    pub department_id: Option<i64>,
    pub designation_id: Option<i64>,
    pub employee_id: Option<String>,
    pub employment_type: Option<String>,
    pub status: Option<String>,
    pub is_super_admin: bool,
    pub email_verified_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<super::department::Department>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<super::designation::Designation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<super::role::Role>>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserSummary,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: i64,         // user_id
    pub email: String,
    pub exp: usize,
    pub iat: usize,
    pub is_super_admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub phone: Option<String>,
    pub department_id: Option<i64>,
    pub designation_id: Option<i64>,
    pub employment_type: Option<String>,
    pub employee_id: Option<String>,
    pub date_of_joining: Option<String>,
    pub work_location: Option<String>,
    pub role_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub photo: Option<String>,
    pub bio: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub department_id: Option<i64>,
    pub designation_id: Option<i64>,
    pub manager_id: Option<i64>,
    pub reporting_manager_id: Option<i64>,
    pub employee_id: Option<String>,
    pub employment_type: Option<String>,
    pub status: Option<String>,
    pub work_location: Option<String>,
    pub account_number: Option<String>,
    pub ifsc_code: Option<String>,
    pub bank_name: Option<String>,
    pub account_type: Option<String>,
    pub pan_number: Option<String>,
    pub esi_number: Option<String>,
    pub pf_number: Option<String>,
    pub aadhar_number: Option<String>,
    
    // Roles
    pub roles: Option<Vec<i64>>,
}

impl User {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            email: row.get("email")?,
            password: row.get("password")?,
            phone: row.get("phone")?,
            avatar: row.get("avatar")?,
            photo: row.get("photo")?,
            bio: row.get("bio")?,
            date_of_birth: row.get("date_of_birth")?,
            gender: row.get("gender")?,
            address: row.get("address")?,
            city: row.get("city")?,
            state: row.get("state")?,
            country: row.get("country")?,
            postal_code: row.get("postal_code")?,
            department_id: row.get("department_id")?,
            designation_id: row.get("designation_id")?,
            manager_id: row.get("manager_id")?,
            reporting_manager_id: row.get("reporting_manager_id")?,
            employee_id: row.get("employee_id")?,
            join_date: row.get("join_date")?,
            date_of_joining: row.get("date_of_joining")?,
            date_of_exit: row.get("date_of_exit")?,
            work_location: row.get("work_location")?,
            employment_type: row.get("employment_type")?,
            status: row.get("status")?,
            timezone: row.get("timezone")?,
            is_super_admin: row.get::<_, Option<bool>>("is_super_admin")?.unwrap_or(false),
            onboarded: row.get::<_, Option<bool>>("onboarded")?.unwrap_or(false),
            account_number: row.get("account_number")?,
            ifsc_code: row.get("ifsc_code")?,
            bank_name: row.get("bank_name")?,
            pan_number: row.get("pan_number")?,
            esi_number: row.get("esi_number")?,
            pf_number: row.get("pf_number")?,
            aadhar_number: row.get("aadhar_number")?,
            email_verified_at: row.get("email_verified_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn to_summary(&self) -> UserSummary {
        UserSummary {
            id: self.id,
            name: self.name.clone(),
            email: self.email.clone(),
            phone: self.phone.clone(),
            avatar: self.avatar.clone(),
            photo: self.photo.clone(),
            department_id: self.department_id,
            designation_id: self.designation_id,
            employee_id: self.employee_id.clone(),
            employment_type: self.employment_type.clone(),
            status: self.status.clone(),
            is_super_admin: self.is_super_admin,
            email_verified_at: self.email_verified_at.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            department: None,
            designation: None,
            roles: None,
        }
    }
}
