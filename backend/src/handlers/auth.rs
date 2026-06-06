use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;

use crate::db::DbPool;
use crate::middleware::auth::{generate_token, get_claims_from_request};
use crate::models::user::{LoginRequest, LoginResponse, User};
use crate::models::{ApiError, ApiResponse};

/// POST /api/auth/login
pub async fn login(
    pool: web::Data<DbPool>,
    jwt_secret: web::Data<Arc<String>>,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    // Find user by email
    let user_result = conn.query_row(
        "SELECT * FROM users WHERE email = ?1 AND deleted_at IS NULL",
        [&body.email],
        User::from_row,
    );

    let user = match user_result {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ApiError::new("Invalid credentials"))
        }
    };

    // Verify password using bcrypt
    // Laravel stores passwords with bcrypt ($2y$ prefix). The bcrypt crate handles $2b$ and $2a$.
    // We need to handle $2y$ → $2b$ conversion.
    let stored_hash = user.password.replace("$2y$", "$2b$");
    let password_valid = bcrypt::verify(&body.password, &stored_hash).unwrap_or(false);

    if !password_valid {
        return HttpResponse::Unauthorized().json(ApiError::new("Invalid credentials"));
    }

    // Get user permissions
    let permissions = crate::middleware::rbac::load_user_permissions(&conn, user.id, user.is_super_admin);

    // Generate JWT token
    let expiration_hours: u64 = std::env::var("JWT_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .unwrap_or(24);

    let token = match generate_token(
        user.id,
        &user.email,
        user.is_super_admin,
        &jwt_secret,
        expiration_hours,
    ) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .json(ApiError::new("Failed to generate token"))
        }
    };

    let refresh_token = uuid::Uuid::new_v4().to_string();
    let refresh_expires = (chrono::Utc::now() + chrono::Duration::days(7))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let _ = conn.execute(
        "INSERT INTO jwt_refresh_tokens (user_id, token, expires_at, created_at, revoked)
         VALUES (?1, ?2, ?3, datetime('now'), 0)",
        rusqlite::params![user.id, &refresh_token, &refresh_expires],
    );

    // Load roles for response
    let roles = load_user_roles(&conn, user.id);
    let mut summary = user.to_summary();
    summary.roles = Some(roles);

    // Load department & designation
    if let Some(dept_id) = summary.department_id {
        summary.department = conn
            .query_row(
                "SELECT * FROM departments WHERE id = ?1",
                [dept_id],
                crate::models::department::Department::from_row,
            )
            .ok();
    }
    if let Some(desg_id) = summary.designation_id {
        summary.designation = conn
            .query_row(
                "SELECT * FROM designations WHERE id = ?1",
                [desg_id],
                crate::models::designation::Designation::from_row,
            )
            .ok();
    }

    let mut settings = std::collections::HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT key, value FROM app_settings") {
        if let Ok(iter) = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default()))
        }) {
            for item in iter.flatten() {
                settings.insert(item.0, item.1);
            }
        }
    }

    #[derive(serde::Serialize)]
    struct LoginResponseExt {
        token: String,
        refresh_token: String,
        user: crate::models::user::UserSummary,
        permissions: Vec<String>,
        settings: std::collections::HashMap<String, String>,
    }

    let response = LoginResponseExt {
        token,
        refresh_token,
        user: summary,
        permissions,
        settings,
    };

    HttpResponse::Ok().json(ApiResponse::success(response))
}

/// GET /api/auth/me — returns current user info
pub async fn me(
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let user = match conn.query_row(
        "SELECT * FROM users WHERE id = ?1 AND deleted_at IS NULL",
        [claims.sub],
        User::from_row,
    ) {
        Ok(u) => u,
        Err(_) => return HttpResponse::NotFound().json(ApiError::new("User not found")),
    };

    let permissions = crate::middleware::rbac::load_user_permissions(&conn, user.id, user.is_super_admin);
    let roles = load_user_roles(&conn, user.id);

    let mut summary = user.to_summary();
    summary.roles = Some(roles);

    if let Some(dept_id) = summary.department_id {
        summary.department = conn
            .query_row(
                "SELECT * FROM departments WHERE id = ?1",
                [dept_id],
                crate::models::department::Department::from_row,
            )
            .ok();
    }
    if let Some(desg_id) = summary.designation_id {
        summary.designation = conn
            .query_row(
                "SELECT * FROM designations WHERE id = ?1",
                [desg_id],
                crate::models::designation::Designation::from_row,
            )
            .ok();
    }

    let mut settings = std::collections::HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT key, value FROM app_settings") {
        if let Ok(iter) = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default()))
        }) {
            for item in iter.flatten() {
                settings.insert(item.0, item.1);
            }
        }
    }

    #[derive(serde::Serialize)]
    struct MeResponse {
        user: crate::models::user::UserSummary,
        permissions: Vec<String>,
        settings: std::collections::HashMap<String, String>,
    }

    HttpResponse::Ok().json(ApiResponse::success(MeResponse {
        user: summary,
        permissions,
        settings,
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
    body: web::Json<RefreshTokenRequest>,
) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let row: Option<(i64, String, bool)> = conn
        .query_row(
            "SELECT user_id, expires_at, revoked FROM jwt_refresh_tokens WHERE token=?1",
            [&body.refresh_token],
            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
        )
        .ok();

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

    let expiration_hours: u64 = std::env::var("JWT_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .unwrap_or(24);

    let token = match generate_token(user.id, &user.email, user.is_super_admin, &jwt_secret, expiration_hours) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Failed to generate token")),
    };

    // Rotate refresh token: revoke old, issue new
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
        rusqlite::params![user_id, &new_refresh, &refresh_expires, &now],
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

// Helper functions

fn load_user_roles(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    user_id: i64,
) -> Vec<crate::models::role::Role> {
    let mut stmt = conn
        .prepare(
            "SELECT r.* FROM roles r
             JOIN role_user ru ON r.id = ru.role_id
             WHERE ru.user_id = ?1",
        )
        .unwrap();

    stmt.query_map([user_id], crate::models::role::Role::from_row)
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}
