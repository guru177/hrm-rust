use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::db::{DbPool, OptionalExt};
use crate::middleware::auth::{generate_token, get_claims_from_request};
use crate::models::organization::{CheckSignupAvailabilityRequest, SignupRequest};
use crate::models::user::{LoginRequest, User};
use crate::models::{ApiError, ApiResponse};
use crate::signup_otp::{self, SendSignupOtpRequest, SignupOtpPayload};
use crate::tenant::{
    load_org_slug, load_organization, normalize_org_slug, resolve_organization_id,
    seed_new_organization_defaults, seed_signup_app_settings, slug_available,
};

fn load_tenant_settings(
    conn: &crate::db::Connection,
    org_id: i64,
) -> std::collections::HashMap<String, String> {
    let mut settings = std::collections::HashMap::new();
    if let Ok(stmt) = conn.prepare(
        "SELECT key, value FROM app_settings WHERE organization_id = ?1",
    ) {
        for (key, value) in stmt.query_map([org_id], |row| {
            Ok((
                row.get_idx::<String>(0)?,
                row.get_idx::<Option<String>>(1)?.unwrap_or_default(),
            ))
        }) {
            settings.insert(key, value);
        }
    }
    settings
}

fn attach_org_to_summary(
    conn: &crate::db::Connection,
    summary: &mut crate::models::user::UserSummary,
) {
    summary.organization = load_organization(conn, summary.organization_id);
}

/// POST /api/auth/login
pub async fn login(
    pool: web::Data<DbPool>,
    jwt_secret: web::Data<Arc<String>>,
    app_config: web::Data<Arc<AppConfig>>,
    req: HttpRequest,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    if let Err(msg) = crate::rate_limit::limit_auth_login(&req, &body.email) {
        return HttpResponse::TooManyRequests().json(ApiError::new(&msg));
    }

    let pool = pool.get_ref().clone();
    let jwt_secret = jwt_secret.into_inner();
    let app_config = app_config.into_inner();
    let body = body.into_inner();
    let client_ip = crate::presence::client_ip(&req);

    match crate::db::runtime::run_db(&pool, move |pool| {
        login_with_pool(pool, &jwt_secret, &app_config, &body, &client_ip)
    })
    .await
    {
        Ok(resp) => resp.into_response(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

fn login_with_pool(
    pool: &DbPool,
    jwt_secret: &Arc<String>,
    app_config: &Arc<AppConfig>,
    body: &LoginRequest,
    client_ip: &str,
) -> crate::db::runtime::BlockJson {
    use crate::db::runtime::BlockJson;

    let conn = match pool.get_platform() {
        Ok(c) => c,
        Err(_) => return BlockJson::error(500, "Database error"),
    };

    let explicit_slug = body
        .org_slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let user = if let Some(slug) = explicit_slug {
        let org_id = match resolve_organization_id(&conn, Some(slug)) {
            Ok(id) => id,
            Err(msg) => return BlockJson::error(400, &msg),
        };
        match conn.query_row(
            "SELECT * FROM users WHERE email = ?1 AND organization_id = ?2 AND deleted_at IS NULL",
            crate::params![body.email, org_id],
            User::from_row,
        ) {
            Ok(u) => u,
            Err(_) => return BlockJson::error(401, "Invalid credentials"),
        }
    } else {
        let candidates_result = conn.query_map_result(
            "SELECT u.* FROM users u
             JOIN organizations o ON o.id = u.organization_id
             WHERE u.email = ?1 AND u.deleted_at IS NULL AND o.status = 'active'",
            crate::params![body.email],
            User::from_row,
        );
        let candidates = match candidates_result {
            Ok(rows) => rows,
            Err(e) => {
                log::error!("[login] query_map_result error for {}: {:?}", body.email, e);
                return BlockJson::error(401, "Invalid credentials");
            }
        };
        log::debug!("[login] found {} candidate(s) for {}", candidates.len(), body.email);
        let valid: Vec<User> = candidates
            .into_iter()
            .filter(|u| {
                let hash = u.password.replace("$2y$", "$2b$");
                bcrypt::verify(&body.password, &hash).unwrap_or(false)
            })
            .collect();
        match valid.len() {
            0 => return BlockJson::error(401, "Invalid credentials"),
            1 => valid.into_iter().next().unwrap(),
            _ => {
                return BlockJson::error(
                    409,
                    "Multiple accounts match this email. Sign in with your organization slug.",
                );
            }
        }
    };


    let org_id = user.organization_id;

    let stored_hash = user.password.replace("$2y$", "$2b$");
    let password_valid = bcrypt::verify(&body.password, &stored_hash).unwrap_or(false);

    if !password_valid {
        return BlockJson::error(401, "Invalid credentials");
    }

    if let Err(msg) = crate::subscription_period::ensure_org_subscription_enforced(&conn, org_id) {
        return BlockJson::error(403, &msg);
    }

    let totp_enabled: bool = conn
        .query_row(
            "SELECT COALESCE(totp_enabled, 0) FROM users WHERE id = ?1",
            [user.id],
            |r| r.get_idx::<i64>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false);

    if totp_enabled {
        let pre_token = match crate::middleware::auth::generate_tenant_pre_auth_token(
            user.id,
            &user.email,
            jwt_secret,
        ) {
            Ok(t) => t,
            Err(_) => return BlockJson::error(500, "Failed to issue 2FA challenge"),
        };
        return BlockJson::ok(serde_json::json!({
            "requires_2fa": true,
            "pre_auth_token": pre_token,
            "user": {
                "id": user.id,
                "name": user.name,
                "email": user.email,
            }
        }));
    }

    complete_login_after_auth(&conn, &user, jwt_secret, app_config, client_ip)
}

/// Issue tenant session after password (and optional 2FA) verification.
pub fn complete_login_after_auth(
    conn: &crate::db::Connection,
    user: &User,
    jwt_secret: &Arc<String>,
    app_config: &Arc<AppConfig>,
    client_ip: &str,
) -> crate::db::runtime::BlockJson {
    use crate::db::runtime::BlockJson;

    let org_id = user.organization_id;
    let (permissions, plan) = crate::plan_limits::resolve_effective_permissions(
        conn,
        org_id,
        crate::middleware::rbac::load_user_permissions(conn, user.id, user.is_super_admin),
    );

    let org_slug = load_org_slug(conn, user.organization_id);
    let token = match generate_token(
        user.id,
        &user.email,
        user.organization_id,
        &org_slug,
        user.is_super_admin,
        jwt_secret,
        app_config.jwt_expiration_hours,
    ) {
        Ok(t) => t,
        Err(_) => return BlockJson::error(500, "Failed to generate token"),
    };

    let refresh_token = uuid::Uuid::new_v4().to_string();
    let refresh_expires = (chrono::Utc::now() + chrono::Duration::days(7))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let _ = conn.execute(
        "INSERT INTO jwt_refresh_tokens (user_id, token, expires_at, created_at, revoked)
         VALUES (?1, ?2, ?3, datetime('now'), 0)",
        crate::params![user.id, &refresh_token, &refresh_expires],
    );

    if user.is_super_admin {
        let _ = crate::presence::upsert_user_presence(
            conn,
            user.id,
            org_id,
            client_ip,
            None,
            None,
            None,
            None,
            None,
        );
    }

    let roles = load_user_roles(conn, user.id);
    let mut summary = user.to_summary();
    summary.roles = Some(roles);
    attach_org_to_summary(conn, &mut summary);

    if let Some(dept_id) = summary.department_id {
        summary.department = conn
            .query_row(
                "SELECT * FROM departments WHERE id = ?1 AND organization_id = ?2",
                crate::params![dept_id, org_id],
                crate::models::department::Department::from_row,
            )
            .ok();
    }
    if let Some(desg_id) = summary.designation_id {
        summary.designation = conn
            .query_row(
                "SELECT * FROM designations WHERE id = ?1 AND organization_id = ?2",
                crate::params![desg_id, org_id],
                crate::models::designation::Designation::from_row,
            )
            .ok();
    }

    let settings = load_tenant_settings(conn, org_id);

    #[derive(serde::Serialize)]
    struct LoginResponseExt {
        token: String,
        refresh_token: String,
        user: crate::models::user::UserSummary,
        permissions: Vec<String>,
        settings: std::collections::HashMap<String, String>,
        plan: Option<crate::plan_limits::OrgPlanInfo>,
    }

    BlockJson::ok(LoginResponseExt {
        token,
        refresh_token,
        user: summary,
        permissions,
        settings,
        plan,
    })
}

/// GET /api/auth/me — returns current user info
pub async fn me(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let org_id = crate::tenant::org_id_from_claims(&claims);

    let user = match conn.query_row(
        "SELECT * FROM users WHERE id = ?1 AND organization_id = ?2 AND deleted_at IS NULL",
        crate::params![claims.sub, org_id],
        User::from_row,
    ) {
        Ok(u) => u,
        Err(_) => return HttpResponse::NotFound().json(ApiError::new("User not found")),
    };

    let (permissions, plan) = crate::plan_limits::resolve_effective_permissions(
        &conn,
        org_id,
        crate::middleware::rbac::load_user_permissions(&conn, user.id, user.is_super_admin),
    );
    let roles = load_user_roles(&conn, user.id);

    let mut summary = user.to_summary();
    summary.roles = Some(roles);
    attach_org_to_summary(&conn, &mut summary);

    if let Some(dept_id) = summary.department_id {
        summary.department = conn
            .query_row(
                "SELECT * FROM departments WHERE id = ?1 AND organization_id = ?2",
                crate::params![dept_id, org_id],
                crate::models::department::Department::from_row,
            )
            .ok();
    }
    if let Some(desg_id) = summary.designation_id {
        summary.designation = conn
            .query_row(
                "SELECT * FROM designations WHERE id = ?1 AND organization_id = ?2",
                crate::params![desg_id, org_id],
                crate::models::designation::Designation::from_row,
            )
            .ok();
    }

    let settings = load_tenant_settings(&conn, org_id);

    if user.is_super_admin {
        let ip = crate::presence::client_ip(&req);
        let _ = crate::presence::upsert_user_presence(
            &conn,
            user.id,
            org_id,
            &ip,
            None,
            None,
            None,
            None,
            None,
        );
    }

    #[derive(serde::Serialize)]
    struct MeResponse {
        user: crate::models::user::UserSummary,
        permissions: Vec<String>,
        settings: std::collections::HashMap<String, String>,
        plan: Option<crate::plan_limits::OrgPlanInfo>,
    }

    HttpResponse::Ok().json(ApiResponse::success(MeResponse {
        user: summary,
        permissions,
        settings,
        plan,
    }))
}

#[derive(serde::Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(serde::Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

/// POST /api/auth/refresh — exchange refresh token for new access token
pub async fn refresh(
    pool: web::Data<DbPool>,
    jwt_secret: web::Data<Arc<String>>,
    app_config: web::Data<Arc<AppConfig>>,
    req: HttpRequest,
    body: web::Json<RefreshTokenRequest>,
) -> HttpResponse {
    if let Err(msg) = crate::rate_limit::limit_auth_refresh(&req) {
        return HttpResponse::TooManyRequests().json(ApiError::new(&msg));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let row: Option<(i64, String, bool)> = conn
        .query_row(
            "SELECT user_id, expires_at, revoked FROM jwt_refresh_tokens WHERE token=?1",
            [&body.refresh_token],
            |r| Ok((r.get_idx::<i64>(0)?, r.get_idx::<String>(1)?, r.get_idx::<i64>(2)? != 0)),
        )
        .optional()
        .unwrap_or(None);

    let Some((user_id, expires_at, revoked)) = row else {
        return HttpResponse::Unauthorized().json(ApiError::new("Invalid refresh token"));
    };
    if revoked {
        return HttpResponse::Unauthorized().json(ApiError::new("Refresh token revoked"));
    }
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if expires_at < now {
        return HttpResponse::Unauthorized().json(ApiError::new("Refresh token expired"));
    }

    let user = match conn.query_row(
        "SELECT * FROM users WHERE id=?1 AND deleted_at IS NULL",
        [user_id],
        User::from_row,
    ) {
        Ok(u) => u,
        Err(_) => return HttpResponse::Unauthorized().json(ApiError::new("User not found")),
    };

    if let Err(msg) =
        crate::subscription_period::ensure_org_subscription_enforced(&conn, user.organization_id)
    {
        return HttpResponse::Forbidden().json(ApiError::new(&msg));
    }

    let org_slug = load_org_slug(&conn, user.organization_id);
    let token = match generate_token(
        user.id,
        &user.email,
        user.organization_id,
        &org_slug,
        user.is_super_admin,
        &jwt_secret,
        app_config.jwt_expiration_hours,
    ) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiError::new("Failed to generate token"))
        }
    };

    let new_refresh = uuid::Uuid::new_v4().to_string();
    let refresh_expires = (chrono::Utc::now() + chrono::Duration::days(7))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let _ = conn.execute(
        "UPDATE jwt_refresh_tokens SET revoked=1 WHERE token=?1",
        [&body.refresh_token],
    );
    let _ = conn.execute(
        "INSERT INTO jwt_refresh_tokens (user_id, token, expires_at, created_at, revoked)
         VALUES (?1, ?2, ?3, ?4, 0)",
        crate::params![user_id, &new_refresh, &refresh_expires, &now],
    );

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "token": token,
        "refresh_token": new_refresh,
    })))
}

/// POST /api/auth/logout — revoke refresh token server-side
pub async fn logout(pool: web::Data<DbPool>, body: Option<web::Json<LogoutRequest>>) -> HttpResponse {
    if let Some(ref req) = body {
        if let Some(ref rt) = req.refresh_token {
            if let Ok(conn) = pool.get() {
                let _ = conn.execute(
                    "UPDATE jwt_refresh_tokens SET revoked=1 WHERE token=?1",
                    [rt],
                );
            }
        }
    }
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "message": "Successfully logged out"
    })))
}

#[derive(serde::Deserialize)]
pub struct PresenceRequest {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub accuracy_meters: Option<f64>,
}

/// POST /api/auth/presence — heartbeat for company admin live tracking
pub async fn presence(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: Option<web::Json<PresenceRequest>>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let org_id = crate::tenant::org_id_from_claims(&claims);

    let is_super_admin: bool = conn
        .query_row(
            "SELECT is_super_admin FROM users WHERE id = ?1 AND organization_id = ?2 AND deleted_at IS NULL",
            crate::params![claims.sub, org_id],
            |row| row.get_idx::<i64>(0).map(|v| v != 0),
        )
        .unwrap_or(false);

    if !is_super_admin {
        return HttpResponse::Forbidden().json(ApiError::new("Only company admins are tracked"));
    }

    let ip = crate::presence::client_ip(&req);
    let latitude = body.as_ref().and_then(|b| b.latitude);
    let longitude = body.as_ref().and_then(|b| b.longitude);
    let city = body.as_ref().and_then(|b| b.city.as_deref());
    let region = body.as_ref().and_then(|b| b.region.as_deref());
    let accuracy_meters = body.as_ref().and_then(|b| b.accuracy_meters);

    if let Err(e) = crate::presence::upsert_user_presence(
        &conn,
        claims.sub,
        org_id,
        &ip,
        latitude,
        longitude,
        city,
        region,
        accuracy_meters,
    ) {
        return HttpResponse::InternalServerError().json(ApiError::new(&format!("{e}")));
    }

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "ok": true,
        "ip_address": ip,
    })))
}

fn admin_email_available(conn: &crate::db::Connection, email: &str) -> bool {
    let normalized = email.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }
    conn.query_row(
        "SELECT 1 FROM users WHERE LOWER(TRIM(email)) = ?1 AND deleted_at IS NULL LIMIT 1",
        crate::params![normalized],
        |_| Ok(()),
    )
    .is_err()
}

fn company_email_available(conn: &crate::db::Connection, email: &str) -> bool {
    let normalized = email.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }
    conn.query_row(
        "SELECT 1 FROM organizations WHERE LOWER(TRIM(company_email)) = ?1 LIMIT 1",
        crate::params![normalized],
        |_| Ok(()),
    )
    .is_err()
}

fn validate_signup_emails(
    conn: &crate::db::Connection,
    company_email: &str,
    admin_email: &str,
) -> Result<(), HttpResponse> {
    if let Err(msg) = crate::validation::validate_email(company_email) {
        return Err(HttpResponse::BadRequest().json(ApiError::new(&msg)));
    }
    if let Err(msg) = crate::validation::validate_email(admin_email) {
        return Err(HttpResponse::BadRequest().json(ApiError::new(&msg)));
    }
    if !company_email_available(conn, company_email) {
        return Err(HttpResponse::Conflict().json(ApiError::new(
            "This company email is already registered to another organization. Use a different email or sign in.",
        )));
    }
    if !admin_email_available(conn, admin_email) {
        return Err(HttpResponse::Conflict().json(ApiError::new(
            "An account with this work email already exists. Sign in or use a different email.",
        )));
    }
    Ok(())
}

fn validate_signup_payload(
    conn: &crate::db::Connection,
    payload: &SignupOtpPayload,
) -> Result<(String, String), HttpResponse> {
    let org_name = payload.organization_name.trim();
    if org_name.is_empty() {
        return Err(HttpResponse::BadRequest().json(ApiError::new("Organization name is required")));
    }

    let slug = normalize_org_slug(&payload.org_slug);
    if slug.len() < 2 {
        return Err(HttpResponse::BadRequest()
            .json(ApiError::new("Organization slug must be at least 2 characters")));
    }
    if slug == "default" {
        return Err(HttpResponse::BadRequest().json(ApiError::new("Organization slug is reserved")));
    }
    if !slug_available(conn, &slug) {
        return Err(HttpResponse::Conflict().json(ApiError::new("Organization slug is already taken")));
    }

    let admin_email = payload.admin_email.trim();
    if admin_email.is_empty() || payload.admin_password.len() < 8 {
        return Err(HttpResponse::BadRequest()
            .json(ApiError::new("Valid admin email and password (min 8 chars) required")));
    }
    if let Err(resp) = validate_signup_emails(conn, payload.company_email.trim(), admin_email) {
        return Err(resp);
    }

    if payload.company_email.trim().is_empty()
        || payload.company_phone.trim().is_empty()
        || payload.contact_person.trim().is_empty()
        || payload.country.trim().is_empty()
        || payload.timezone.trim().is_empty()
        || payload.admin_mobile.trim().is_empty()
        || payload.admin_name.trim().is_empty()
    {
        return Err(HttpResponse::BadRequest()
            .json(ApiError::new("All company and admin fields are required")));
    }

    Ok((org_name.to_string(), slug))
}

fn validate_signup_fields(
    conn: &crate::db::Connection,
    body: &SignupRequest,
) -> Result<(String, String), HttpResponse> {
    let org_name = body.organization_name.trim();
    if org_name.is_empty() {
        return Err(HttpResponse::BadRequest().json(ApiError::new("Organization name is required")));
    }

    let slug = normalize_org_slug(&body.org_slug);
    if slug.len() < 2 {
        return Err(HttpResponse::BadRequest()
            .json(ApiError::new("Organization slug must be at least 2 characters")));
    }
    if slug == "default" {
        return Err(HttpResponse::BadRequest().json(ApiError::new("Organization slug is reserved")));
    }
    if !slug_available(conn, &slug) {
        return Err(HttpResponse::Conflict().json(ApiError::new("Organization slug is already taken")));
    }

    let admin_email = body.admin_email.trim();
    if admin_email.is_empty() || body.admin_password.len() < 8 {
        return Err(HttpResponse::BadRequest()
            .json(ApiError::new("Valid admin email and password (min 8 chars) required")));
    }

    if body.admin_password != body.confirm_password {
        return Err(HttpResponse::BadRequest().json(ApiError::new("Passwords do not match")));
    }

    let company_email = body.company_email.trim();
    if let Err(resp) = validate_signup_emails(conn, company_email, admin_email) {
        return Err(resp);
    }

    let company_phone = body.company_phone.trim();
    let contact_person = body.contact_person.trim();
    let country = body.country.trim();
    let timezone = body.timezone.trim();
    let admin_mobile = body.admin_mobile.trim();

    if company_email.is_empty()
        || company_phone.is_empty()
        || contact_person.is_empty()
        || country.is_empty()
        || timezone.is_empty()
        || admin_mobile.is_empty()
        || body.admin_name.trim().is_empty()
    {
        return Err(HttpResponse::BadRequest()
            .json(ApiError::new("All company and admin fields are required")));
    }

    Ok((org_name.to_string(), slug))
}

fn finalize_signup_response(
    conn: &crate::db::Connection,
    user_id: i64,
    org_id: i64,
    admin_email: &str,
    slug: &str,
    jwt_secret: &Arc<String>,
    jwt_expiration_hours: u64,
) -> HttpResponse {
    let token = match generate_token(
        user_id,
        admin_email,
        org_id,
        slug,
        true,
        jwt_secret,
        jwt_expiration_hours,
    ) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .json(ApiError::new("Failed to generate token"))
        }
    };

    let user = match conn.query_row("SELECT * FROM users WHERE id = ?1", [user_id], User::from_row) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiError::new("Failed to load user"))
        }
    };

    let mut summary = user.to_summary();
    attach_org_to_summary(conn, &mut summary);
    let settings = load_tenant_settings(conn, org_id);
    let (permissions, plan) =
        crate::plan_limits::resolve_effective_permissions(conn, org_id, vec!["*".to_string()]);

    let refresh_token = uuid::Uuid::new_v4().to_string();
    let refresh_expires = (chrono::Utc::now() + chrono::Duration::days(7))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let _ = conn.execute(
        "INSERT INTO jwt_refresh_tokens (user_id, token, expires_at, created_at, revoked)
         VALUES (?1, ?2, ?3, datetime('now'), 0)",
        crate::params![user_id, &refresh_token, &refresh_expires],
    );

    HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
        "token": token,
        "refresh_token": refresh_token,
        "user": summary,
        "permissions": permissions,
        "plan": plan,
        "settings": settings,
        "message": "Organization created successfully",
    })))
}

fn create_organization_from_payload(
    conn: &crate::db::Connection,
    payload: &SignupOtpPayload,
    org_name: &str,
    slug: &str,
) -> Result<(i64, i64), HttpResponse> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let admin_email = payload.admin_email.trim();

    if conn
        .execute(
            "INSERT INTO organizations (name, slug, status, plan, company_email, company_phone, contact_person, country, timezone, created_at, updated_at)
             VALUES (?1, ?2, 'active', 'trial', ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            crate::params![
                org_name,
                slug,
                payload.company_email.trim(),
                payload.company_phone.trim(),
                payload.contact_person.trim(),
                payload.country.trim(),
                payload.timezone.trim(),
                &now
            ],
        )
        .is_err()
    {
        return Err(HttpResponse::InternalServerError()
            .json(ApiError::new("Failed to create organization")));
    }

    let org_id = conn.last_insert_rowid();
    let _ = crate::subscription_period::assign_org_subscription(conn, org_id, "trial");

    let hashed = match bcrypt::hash(&payload.admin_password, 12) {
        Ok(h) => h,
        Err(_) => {
            return Err(HttpResponse::InternalServerError()
                .json(ApiError::new("Failed to hash password")))
        }
    };

    if conn
        .execute(
            "INSERT INTO users (name, email, password, organization_id, is_super_admin, phone, timezone, country, email_verified_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, ?8, ?8)",
            crate::params![
                payload.admin_name.trim(),
                admin_email,
                hashed,
                org_id,
                payload.admin_mobile.trim(),
                payload.timezone.trim(),
                payload.country.trim(),
                &now
            ],
        )
        .is_err()
    {
        let _ = conn.execute("DELETE FROM organizations WHERE id = ?1", [org_id]);
        return Err(HttpResponse::Conflict().json(ApiError::new("Admin email could not be registered")));
    }

    let user_id = conn.last_insert_rowid();
    seed_new_organization_defaults(conn, org_id);
    let shift_from = now.get(0..10).unwrap_or(&now).to_string();
    let _ = crate::shift_logic::assign_general_shift_to_user(conn, user_id, &shift_from);
    seed_signup_app_settings(
        conn,
        org_id,
        org_name,
        &payload.contact_person,
        &payload.company_email,
        &payload.company_phone,
        &payload.country,
        &payload.timezone,
    );

    Ok((org_id, user_id))
}

/// POST /api/public/signup/check-availability — early slug / email checks before OTP step
pub async fn check_signup_availability(
    pool: web::Data<DbPool>,
    body: web::Json<CheckSignupAvailabilityRequest>,
) -> HttpResponse {
    if !crate::config::AppConfig::public_signup_enabled() {
        return HttpResponse::Forbidden().json(ApiError::new(
            "Public signup is disabled. Contact your administrator for access.",
        ));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    if let Some(raw_slug) = body.org_slug.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let slug = normalize_org_slug(raw_slug);
        if slug.len() < 2 {
            return HttpResponse::BadRequest().json(ApiError::new(
                "Organization slug must be at least 2 characters",
            ));
        }
        if slug == "default" {
            return HttpResponse::BadRequest().json(ApiError::new("Organization slug is reserved"));
        }
        if !slug_available(&conn, &slug) {
            return HttpResponse::Conflict()
                .json(ApiError::new("Organization slug is already taken"));
        }
    }

    if let Some(email) = body
        .company_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Err(msg) = crate::validation::validate_email(email) {
            return HttpResponse::BadRequest().json(ApiError::new(&msg));
        }
        if !company_email_available(&conn, email) {
            return HttpResponse::Conflict().json(ApiError::new(
                "This company email is already registered to another organization. Use a different email or sign in.",
            ));
        }
    }

    if let Some(email) = body.admin_email.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if let Err(msg) = crate::validation::validate_email(email) {
            return HttpResponse::BadRequest().json(ApiError::new(&msg));
        }
        if !admin_email_available(&conn, email) {
            return HttpResponse::Conflict().json(ApiError::new(
                "An account with this work email already exists. Sign in or use a different email.",
            ));
        }
    }

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({ "available": true })))
}

/// POST /api/public/signup/send-otp — validate signup form and send email/WhatsApp OTP
pub async fn send_signup_otp(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<SendSignupOtpRequest>,
) -> HttpResponse {
    if !crate::config::AppConfig::public_signup_enabled() {
        return HttpResponse::Forbidden().json(ApiError::new(
            "Public signup is disabled. Contact your administrator for access.",
        ));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    if let Err(resp) = validate_signup_fields(&conn, &body.signup) {
        return resp;
    }

    let channel = body.channel.trim().to_lowercase();
    if channel != "email" && channel != "whatsapp" {
        return HttpResponse::BadRequest().json(ApiError::new("Channel must be email or whatsapp"));
    }

    let payload = SignupOtpPayload::from_request(&body.signup);
    let destination = match signup_otp::destination_for_channel(&channel, &payload) {
        Some(d) if !d.trim().is_empty() => d,
        _ => {
            return HttpResponse::BadRequest()
                .json(ApiError::new("Missing destination for selected channel"))
        }
    };

    if let Err(msg) = crate::rate_limit::limit_signup_otp_send(&req, &destination) {
        return HttpResponse::TooManyRequests().json(ApiError::new(&msg));
    }

    let otp = signup_otp::generate_otp();
    let verification_id = match signup_otp::store_challenge(&conn, &channel, &destination, &otp, &payload)
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiError::new(&format!("Failed to store verification: {e}")))
        }
    };

    if let Err(e) = signup_otp::dispatch_otp(&channel, &destination, &otp).await {
        let _ = conn.execute(
            "DELETE FROM signup_otp_challenges WHERE id = ?1",
            [&verification_id],
        );
        return HttpResponse::BadGateway().json(ApiError::new(&e));
    }

    let masked = if channel == "email" {
        signup_otp::mask_email(&destination)
    } else {
        signup_otp::mask_phone(&destination)
    };

    let mut data = serde_json::json!({
        "verification_id": verification_id,
        "channel": channel,
        "destination_masked": masked,
        "expires_in": 600,
        "message": "Verification code sent",
    });

    if signup_otp::signup_otp_debug_enabled() {
        data["debug_otp"] = serde_json::json!(otp);
    }

    HttpResponse::Ok().json(ApiResponse::success(data))
}

/// POST /api/public/signup — create organization + first tenant admin
pub async fn signup(
    pool: web::Data<DbPool>,
    jwt_secret: web::Data<Arc<String>>,
    app_config: web::Data<Arc<AppConfig>>,
    req: HttpRequest,
    body: web::Json<SignupRequest>,
) -> HttpResponse {
    if !crate::config::AppConfig::public_signup_enabled() {
        return HttpResponse::Forbidden().json(ApiError::new(
            "Public signup is disabled. Contact your administrator for access.",
        ));
    }
    if let Err(msg) = crate::rate_limit::limit_public_signup(&req) {
        return HttpResponse::TooManyRequests().json(ApiError::new(&msg));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let payload = if signup_otp::signup_otp_required() {
        let verification_id = body
            .verification_id
            .as_deref()
            .filter(|s| !s.trim().is_empty());
        let otp = body.otp.as_deref().filter(|s| !s.trim().is_empty());
        let (Some(verification_id), Some(otp)) = (verification_id, otp) else {
            return HttpResponse::BadRequest().json(ApiError::new(
                "Email or WhatsApp verification is required before signup. Request a code first.",
            ));
        };
        match signup_otp::verify_and_consume(&conn, verification_id, otp.trim()) {
            Ok(p) => p,
            Err(e) => return HttpResponse::BadRequest().json(ApiError::new(&e)),
        }
    } else {
        if let Err(resp) = validate_signup_fields(&conn, &body) {
            return resp;
        }
        SignupOtpPayload::from_request(&body)
    };

    let (org_name, slug) = match validate_signup_payload(&conn, &payload) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let admin_email = payload.admin_email.trim();
    let (org_id, user_id) = match create_organization_from_payload(&conn, &payload, &org_name, &slug) {
        Ok(ids) => ids,
        Err(resp) => return resp,
    };

    finalize_signup_response(
        &conn,
        user_id,
        org_id,
        admin_email,
        &slug,
        &jwt_secret,
        app_config.jwt_expiration_hours,
    )
}

fn load_user_roles(
    conn: &crate::db::Connection,
    user_id: i64,
) -> Vec<crate::models::role::Role> {
    let Ok(stmt) = conn.prepare(
        "SELECT r.* FROM roles r
         JOIN role_user ru ON r.id = ru.role_id
         WHERE ru.user_id = ?1",
    ) else {
        return Vec::new();
    };

    stmt.query_map([user_id], crate::models::role::Role::from_row)
}

#[derive(Debug, serde::Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
    pub org_slug: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ResetPasswordRequest {
    pub email: Option<String>,
    pub token: Option<String>,
    pub verification_id: Option<String>,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct VerifyPasswordResetOtpRequest {
    pub verification_id: String,
    pub otp: String,
}

const FORGOT_PASSWORD_OTP_MESSAGE: &str = "If an account exists, a verification code has been sent.";

fn active_users_for_email(
    conn: &crate::db::Connection,
    email: &str,
    org_id: Option<i64>,
) -> Vec<(i64, i64)> {
    if let Some(org_id) = org_id {
        return conn.query_map(
            "SELECT u.id, u.organization_id FROM users u
             JOIN organizations o ON o.id = u.organization_id
             WHERE u.email = ?1 AND u.organization_id = ?2
               AND u.deleted_at IS NULL AND o.status = 'active'",
            crate::params![email, org_id],
            |row| Ok((row.get_idx::<i64>(0)?, row.get_idx::<i64>(1)?)),
        );
    }

    conn.query_map(
        "SELECT u.id, u.organization_id FROM users u
         JOIN organizations o ON o.id = u.organization_id
         WHERE u.email = ?1 AND u.deleted_at IS NULL AND o.status = 'active'",
        crate::params![email],
        |row| Ok((row.get_idx::<i64>(0)?, row.get_idx::<i64>(1)?)),
    )
}

async fn issue_password_reset_otp(
    conn: &crate::db::Connection,
    user_id: i64,
    org_id: i64,
    email: &str,
) -> Result<(String, String), String> {
    let otp = crate::signup_otp::generate_otp();
    let verification_id =
        crate::password_reset_otp::store_challenge(conn, user_id, org_id, email, &otp)?;
    let org_name = load_organization(conn, org_id)
        .map(|o| o.name)
        .unwrap_or_else(|| "your organization".to_string());
    crate::password_reset_otp::send_otp_email(conn, org_id, email, &otp, &org_name).await?;
    let masked = crate::password_reset_otp::mask_email(email);
    Ok((verification_id, masked))
}

/// POST /api/auth/forgot-password
pub async fn forgot_password(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<ForgotPasswordRequest>,
) -> HttpResponse {
    let email = body.email.trim();
    if email.is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("Email is required"));
    }

    if let Err(msg) = crate::rate_limit::limit_password_reset(&req, email) {
        return HttpResponse::TooManyRequests().json(ApiError::new(&msg));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let explicit_slug = body
        .org_slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let org_id = if let Some(slug) = explicit_slug {
        match resolve_organization_id(&conn, Some(slug)) {
            Ok(id) => Some(id),
            Err(msg) => {
                return HttpResponse::BadRequest().json(ApiError::new(&msg));
            }
        }
    } else {
        None
    };

    let matches = active_users_for_email(&conn, email, org_id);

    if matches.len() > 1 && org_id.is_none() {
        return HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": FORGOT_PASSWORD_OTP_MESSAGE,
            "requires_org_slug": true,
        })));
    }

    let Some((user_id, matched_org_id)) = matches.into_iter().next() else {
        if org_id.is_some() {
            return HttpResponse::NotFound().json(ApiError::new(
                "No account found with this email in this organization.",
            ));
        }
        return HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": FORGOT_PASSWORD_OTP_MESSAGE,
        })));
    };

    match issue_password_reset_otp(&conn, user_id, matched_org_id, email).await {
        Ok((verification_id, masked_email)) => {
            let mut data = serde_json::json!({
                "message": format!("Verification code sent to {masked_email}."),
                "verification_id": verification_id,
                "masked_email": masked_email,
                "account_found": true,
            });
            if crate::password_reset_otp::debug_enabled() {
                data["debug_note"] = serde_json::json!("Check backend logs for OTP when SMTP debug is enabled");
            }
            HttpResponse::Ok().json(ApiResponse::success(data))
        }
        Err(e) => {
            log::warn!("password reset OTP email failed for {}: {}", email, e);
            // Do not reveal SMTP failures to clients (anti-enumeration + SEC-11).
            HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                "message": FORGOT_PASSWORD_OTP_MESSAGE,
            })))
        }
    }
}

/// POST /api/auth/verify-password-reset-otp
pub async fn verify_password_reset_otp(
    pool: web::Data<DbPool>,
    body: web::Json<VerifyPasswordResetOtpRequest>,
) -> HttpResponse {
    let verification_id = body.verification_id.trim();
    let otp = body.otp.trim();
    if verification_id.is_empty() || otp.is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("Verification id and code are required"));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    match crate::password_reset_otp::verify_otp(&conn, verification_id, otp) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "verified": true,
            "verification_id": verification_id,
        }))),
        Err(msg) => HttpResponse::BadRequest().json(ApiError::new(&msg)),
    }
}

/// POST /api/auth/reset-password
pub async fn reset_password(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<ResetPasswordRequest>,
) -> HttpResponse {
    let email = body.email.as_deref().unwrap_or("").trim();
    let verification_id = body.verification_id.as_deref().unwrap_or("").trim();
    let token = body.token.as_deref().unwrap_or("").trim();

    let rate_key = if !verification_id.is_empty() {
        verification_id
    } else if !email.is_empty() {
        email
    } else {
        return HttpResponse::BadRequest().json(ApiError::new(
            "Verification session or reset token is required",
        ));
    };

    if let Err(msg) = crate::rate_limit::limit_password_reset(&req, rate_key) {
        return HttpResponse::TooManyRequests().json(ApiError::new(&msg));
    }

    if body.password.len() < 8 {
        return HttpResponse::BadRequest().json(ApiError::new("Password must be at least 8 characters"));
    }
    if body.password != body.password_confirmation {
        return HttpResponse::BadRequest().json(ApiError::new("Password confirmation does not match"));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let user_id = if !verification_id.is_empty() {
        match crate::password_reset_otp::consume_verified_challenge(&conn, verification_id) {
            Ok(id) => id,
            Err(msg) => return HttpResponse::BadRequest().json(ApiError::new(&msg)),
        }
    } else if !email.is_empty() && !token.is_empty() {
        match crate::password_reset::consume_token(&conn, email, token) {
            Ok(id) => id,
            Err(msg) => return HttpResponse::BadRequest().json(ApiError::new(&msg)),
        }
    } else {
        return HttpResponse::BadRequest().json(ApiError::new(
            "Verification session or reset token is required",
        ));
    };

    let new_hash = match bcrypt::hash(&body.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiError::new("Failed to hash password"))
        }
    };

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if conn
        .execute(
            "UPDATE users SET password = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
            crate::params![&new_hash, &now, user_id],
        )
        .is_err()
    {
        return HttpResponse::InternalServerError().json(ApiError::new("Failed to update password"));
    }

    let _ = conn.execute(
        "UPDATE jwt_refresh_tokens SET revoked = 1 WHERE user_id = ?1 AND revoked = 0",
        [user_id],
    );

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "message": "Password updated successfully",
    })))
}
