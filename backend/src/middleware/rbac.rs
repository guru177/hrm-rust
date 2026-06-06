use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::ErrorForbidden;
use actix_web::http::Method;
use actix_web::{middleware::Next, Error, HttpResponse};

use crate::db::DbPool;
use crate::middleware::auth::extract_claims;
use crate::models::ApiError;

/// Load permission slugs for a user (super admin gets `*`).
pub fn load_user_permissions(
    conn: &rusqlite::Connection,
    user_id: i64,
    is_super_admin: bool,
) -> Vec<String> {
    if is_super_admin {
        return vec!["*".to_string()];
    }
    let mut stmt = match conn.prepare(
        "SELECT DISTINCT p.slug FROM permissions p
         JOIN permission_role pr ON p.id = pr.permission_id
         JOIN role_user ru ON pr.role_id = ru.role_id
         WHERE ru.user_id = ?1",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([user_id], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn has_permission(permissions: &[String], slug: &str) -> bool {
    permissions.iter().any(|p| p == "*" || p == slug)
}

/// Returns required permission slug, or `None` if any authenticated user may access.
pub fn required_permission(method: &Method, path: &str) -> Option<&'static str> {
    if path.starts_with("/api/onboarding/")
        || path.starts_with("/api/admin/settings/profile")
        || path.starts_with("/api/admin/settings/password")
    {
        return None;
    }
    if path.starts_with("/api/admin/settings/app") || path.starts_with("/api/admin/settings/centers")
        || path.starts_with("/api/admin/api/settings/centers")
    {
        return Some("manage-settings");
    }
    if path.contains("/leave-requests/manage")
        || (path.contains("/leave-requests/")
            && (path.contains("/approve")
                || path.contains("/reject")
                || path.contains("/remarks")))
    {
        return Some("manage-leave-requests");
    }
    if path.starts_with("/api/admin/dashboard") {
        return Some("view-dashboard");
    }
    if path == "/api/admin/users/list" && method == Method::GET {
        return None;
    }
    if (path == "/api/admin/payroll/preview" || path == "/api/admin/payroll/generate")
        && method == Method::POST
    {
        return Some("manage-payroll");
    }
    if path.contains("/payslips/") && path.ends_with("/unlock") && method == Method::POST {
        return Some("manage-payroll");
    }
    if path.contains("/salary-structure") && method != Method::GET {
        return Some("manage-payroll");
    }
    if path.starts_with("/api/admin/users") || path.starts_with("/api/admin/roles") {
        return Some(if method == Method::GET {
            "view-users"
        } else {
            "create-users"
        });
    }
    if path.starts_with("/api/admin/permissions") {
        return Some("view-users");
    }
    if path.starts_with("/api/admin/departments") {
        return Some("view-departments");
    }
    if path.starts_with("/api/admin/designations") {
        return Some("view-designations");
    }
    if path.starts_with("/api/admin/careers") || path.starts_with("/api/admin/job-applications") {
        return Some("view-jobs");
    }
    if path.starts_with("/api/admin/biometric") {
        return Some("view-attendance");
    }
    if path.starts_with("/api/admin/attendance/users") {
        return Some("view-attendance");
    }
    if path.starts_with("/api/admin/attendance") {
        return Some("view-attendance");
    }
    if path.starts_with("/api/admin/shifts") {
        return Some("view-attendance");
    }
    if path.starts_with("/api/admin/leave-requests") {
        return Some("manage-leave-requests");
    }
    if path.starts_with("/api/admin/me/payslips") {
        return Some("view-payroll");
    }
    if path.starts_with("/api/admin/me/") {
        return None;
    }
    if path.starts_with("/api/admin/holidays") {
        return Some("view-holidays");
    }
    if path.starts_with("/api/admin/salaries")
        || path.starts_with("/api/admin/payroll")
        || path.starts_with("/api/admin/payslips")
        || path.starts_with("/api/admin/reports")
    {
        return Some("view-payroll");
    }
    if path.starts_with("/api/admin/workflows") {
        return Some("view-workflows");
    }
    if path.starts_with("/api/admin/tasks") {
        return Some("view-tasks");
    }
    if path.starts_with("/api/admin/projects") {
        return Some("view-projects");
    }
    None
}

pub async fn rbac_middleware<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<BoxBody, B>>, Error>
where
    B: MessageBody + 'static,
{
    let path = req.path().to_string();
    if !path.starts_with("/api/admin") {
        let res = next.call(req).await?;
        return Ok(res.map_into_right_body());
    }

    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    if let Some(slug) = required_permission(req.method(), &path) {
        let pool = req
            .app_data::<actix_web::web::Data<DbPool>>()
            .ok_or_else(|| ErrorForbidden("Server configuration error"))?;
        let conn = pool
            .get()
            .map_err(|_| ErrorForbidden("Database unavailable"))?;
        let perms = load_user_permissions(&conn, claims.sub, claims.is_super_admin);
        if !has_permission(&perms, slug) {
            let body = HttpResponse::Forbidden().json(ApiError::new(&format!(
                "Missing permission: {}",
                slug
            )));
            return Ok(req.into_response(body.map_into_boxed_body()).map_into_left_body());
        }
    }

    let res = next.call(req).await?;
    Ok(res.map_into_right_body())
}
