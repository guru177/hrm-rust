use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};
use crate::models::user::{User, CreateUserRequest, UpdateUserRequest, UserSummary};
use crate::storage;
use futures_util::StreamExt;
use std::collections::HashMap;

fn load_user_summary(conn: &rusqlite::Connection, user_id: i64) -> Option<UserSummary> {
    let user = conn
        .query_row(
            "SELECT * FROM users WHERE id = ?1 AND deleted_at IS NULL",
            [user_id],
            User::from_row,
        )
        .ok()?;
    let mut summary = user.to_summary();
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
    let mut stmt = conn
        .prepare("SELECT r.* FROM roles r JOIN role_user ru ON r.id = ru.role_id WHERE ru.user_id = ?1")
        .ok()?;
    let roles: Vec<crate::models::role::Role> = stmt
        .query_map([user_id], crate::models::role::Role::from_row)
        .ok()?
        .filter_map(|r| r.ok())
        .collect();
    summary.roles = Some(roles);
    Some(summary)
}

fn opt_i64(s: &str) -> Option<i64> {
    if s.trim().is_empty() {
        None
    } else {
        s.trim().parse().ok()
    }
}

fn parse_roles(raw: &str) -> Option<Vec<i64>> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    if t.starts_with('[') {
        return serde_json::from_str(t).ok();
    }
    let ids: Vec<i64> = t.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    if ids.is_empty() {
        None
    } else {
        Some(ids)
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn normalize_string_field(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn employee_id_taken(
    conn: &rusqlite::Connection,
    employee_id: &str,
    exclude_user_id: Option<i64>,
) -> bool {
    match exclude_user_id {
        Some(uid) => conn
            .query_row(
                "SELECT 1 FROM users WHERE employee_id=?1 AND deleted_at IS NULL AND id!=?2",
                rusqlite::params![employee_id, uid],
                |_| Ok(()),
            )
            .is_ok(),
        None => conn
            .query_row(
                "SELECT 1 FROM users WHERE employee_id=?1 AND deleted_at IS NULL",
                [employee_id],
                |_| Ok(()),
            )
            .is_ok(),
    }
}

/// GET /api/admin/users
pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let query_string = req.query_string();
    let params: Vec<(String, String)> = serde_urlencoded::from_str(query_string).unwrap_or_default();
    let search = params.iter().find(|(k, _)| k == "search").map(|(_, v)| v.clone());
    let page: i64 = params.iter().find(|(k, _)| k == "page").and_then(|(_, v)| v.parse().ok()).unwrap_or(1);
    let per_page: i64 = params.iter().find(|(k, _)| k == "per_page").and_then(|(_, v)| v.parse().ok()).unwrap_or(15);
    let offset = (page - 1) * per_page;

    let (where_clause, search_param) = if let Some(ref s) = search {
        ("WHERE u.deleted_at IS NULL AND (u.name LIKE ?1 OR u.email LIKE ?1)".to_string(), format!("%{}%", s))
    } else {
        ("WHERE u.deleted_at IS NULL".to_string(), String::new())
    };

    let total: i64 = if search.is_some() {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM users u {}", where_clause),
            [&search_param],
            |row| row.get(0),
        ).unwrap_or(0)
    } else {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM users u {}", where_clause),
            [],
            |row| row.get(0),
        ).unwrap_or(0)
    };

    let sql = format!(
        "SELECT u.* FROM users u {} ORDER BY u.created_at DESC LIMIT ?2 OFFSET ?3",
        where_clause
    );

    let users: Vec<serde_json::Value> = if search.is_some() {
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(rusqlite::params![&search_param, per_page, offset], |row| {
            let user = User::from_row(row)?;
            Ok(user)
        }).unwrap().filter_map(|r| r.ok()).map(|u| {
            let mut summary = u.to_summary();
            // Load department
            if let Some(dept_id) = summary.department_id {
                summary.department = conn.query_row(
                    "SELECT * FROM departments WHERE id = ?1", [dept_id],
                    crate::models::department::Department::from_row,
                ).ok();
            }
            if let Some(desg_id) = summary.designation_id {
                summary.designation = conn.query_row(
                    "SELECT * FROM designations WHERE id = ?1", [desg_id],
                    crate::models::designation::Designation::from_row,
                ).ok();
            }
            serde_json::to_value(summary).unwrap()
        }).collect()
    } else {
        let mut stmt = conn.prepare(
            &format!("SELECT u.* FROM users u {} ORDER BY u.created_at DESC LIMIT ?1 OFFSET ?2", where_clause)
        ).unwrap();
        stmt.query_map(rusqlite::params![per_page, offset], |row| {
            let user = User::from_row(row)?;
            Ok(user)
        }).unwrap().filter_map(|r| r.ok()).map(|u| {
            let mut summary = u.to_summary();
            if let Some(dept_id) = summary.department_id {
                summary.department = conn.query_row(
                    "SELECT * FROM departments WHERE id = ?1", [dept_id],
                    crate::models::department::Department::from_row,
                ).ok();
            }
            if let Some(desg_id) = summary.designation_id {
                summary.designation = conn.query_row(
                    "SELECT * FROM designations WHERE id = ?1", [desg_id],
                    crate::models::designation::Designation::from_row,
                ).ok();
            }
            serde_json::to_value(summary).unwrap()
        }).collect()
    };

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "type": "success",
        "data": users,
        "total": total,
        "page": page,
        "per_page": per_page,
    }))
}

/// GET /api/admin/users/{id}
pub async fn show(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let user_id = path.into_inner();
    match load_user_summary(&conn, user_id) {
        Some(summary) => HttpResponse::Ok().json(ApiResponse::success(summary)),
        None => HttpResponse::NotFound().json(ApiError::new("User not found")),
    }
}

/// POST /api/admin/users
pub async fn store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<CreateUserRequest>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let hashed = bcrypt::hash(&body.password, 12).unwrap();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let employee_id = normalize_optional_string(body.employee_id.clone());
    if let Some(ref eid) = employee_id {
        if employee_id_taken(&conn, eid, None) {
            return HttpResponse::BadRequest()
                .json(ApiError::new("Employee ID is already assigned to another user"));
        }
    }

    let result = conn.execute(
        "INSERT INTO users (name, email, password, phone, department_id, designation_id, employment_type, employee_id, date_of_joining, work_location, email_verified_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            body.name, body.email, hashed, body.phone,
            body.department_id, body.designation_id,
            body.employment_type.as_deref().unwrap_or("full-time"),
            employee_id, body.date_of_joining, body.work_location,
            &now, &now, &now,
        ],
    );

    match result {
        Ok(_) => {
            let user_id = conn.last_insert_rowid();

            // Assign roles if provided
            if let Some(ref role_ids) = body.role_ids {
                for role_id in role_ids {
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO role_user (user_id, role_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![user_id, role_id, &now, &now],
                    );
                }
            }

            let shift_from = body
                .date_of_joining
                .as_deref()
                .filter(|d| !d.is_empty())
                .map(|d| d.to_string())
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
            let _ = crate::shift_logic::assign_general_shift_to_user(&conn, user_id, &shift_from);

            HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
                "id": user_id,
                "message": "User created successfully"
            })))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed to create user: {}", e))),
    }
}

/// PUT /api/admin/users/{id}
pub async fn update(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>, body: web::Json<UpdateUserRequest>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let user_id = path.into_inner();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Build dynamic UPDATE query
    let mut sets = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    macro_rules! maybe_set {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = body.$field {
                sets.push(format!("{} = ?{}", $col, idx));
                params.push(Box::new(val.clone()));
                idx += 1;
            }
        };
    }

    maybe_set!(name, "name");
    maybe_set!(email, "email");
    maybe_set!(phone, "phone");
    maybe_set!(avatar, "avatar");
    maybe_set!(photo, "photo");
    maybe_set!(bio, "bio");
    maybe_set!(date_of_birth, "date_of_birth");
    maybe_set!(gender, "gender");
    maybe_set!(address, "address");
    maybe_set!(city, "city");
    maybe_set!(state, "state");
    maybe_set!(country, "country");
    maybe_set!(postal_code, "postal_code");
    maybe_set!(employment_type, "employment_type");
    maybe_set!(status, "status");
    maybe_set!(work_location, "work_location");
    if body.employee_id.is_some() {
        let employee_id = normalize_optional_string(body.employee_id.clone());
        if let Some(ref eid) = employee_id {
            if employee_id_taken(&conn, eid, Some(user_id)) {
                return HttpResponse::BadRequest()
                    .json(ApiError::new("Employee ID is already assigned to another user"));
            }
        }
        sets.push(format!("employee_id = ?{}", idx));
        params.push(Box::new(employee_id));
        idx += 1;
    }
    maybe_set!(account_number, "account_number");
    maybe_set!(ifsc_code, "ifsc_code");
    maybe_set!(bank_name, "bank_name");
    maybe_set!(pan_number, "pan_number");

    if let Some(ref val) = body.account_type {
        sets.push(format!("account_type = ?{}", idx));
        params.push(Box::new(val.clone()));
        idx += 1;
    }

    if let Some(dept_id) = body.department_id {
        sets.push(format!("department_id = ?{}", idx));
        params.push(Box::new(dept_id));
        idx += 1;
    }
    if let Some(desg_id) = body.designation_id {
        sets.push(format!("designation_id = ?{}", idx));
        params.push(Box::new(desg_id));
        idx += 1;
    }

    if sets.is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("No fields to update"));
    }

    sets.push(format!("updated_at = ?{}", idx));
    let now_for_roles = now.clone();
    params.push(Box::new(now));
    idx += 1;

    params.push(Box::new(user_id));

    let sql = format!(
        "UPDATE users SET {} WHERE id = ?{}",
        sets.join(", "),
        idx
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    match conn.execute(&sql, param_refs.as_slice()) {
        Ok(_) => {
            // Update roles if provided
            if let Some(ref roles) = body.roles {
                let _ = conn.execute("DELETE FROM role_user WHERE user_id = ?1", rusqlite::params![user_id]);
                for role_id in roles {
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO role_user (user_id, role_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![user_id, role_id, &now_for_roles, &now_for_roles],
                    );
                }
            }

            match load_user_summary(&conn, user_id) {
                Some(summary) => HttpResponse::Ok().json(ApiResponse::success(summary)),
                None => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "message": "User updated successfully"
                }))),
            }
        },
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed to update user: {}", e))),
    }
}

/// POST /api/admin/users/{id} — multipart form (profile photo + fields)
pub async fn update_form(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    mut payload: Multipart,
) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let user_id = path.into_inner();
    if load_user_summary(&conn, user_id).is_none() {
        return HttpResponse::NotFound().json(ApiError::new("User not found"));
    }

    let mut fields: HashMap<String, String> = HashMap::new();
    let mut roles: Vec<i64> = Vec::new();
    let mut photo_data: Option<(Option<String>, Option<String>, Vec<u8>)> = None;
    let mut remove_photo = false;

    while let Some(field) = payload.next().await {
        let mut field = match field {
            Ok(f) => f,
            Err(e) => {
                return HttpResponse::BadRequest().json(ApiError::new(&format!("Upload error: {}", e)));
            }
        };
        let name = field.name().unwrap_or("").to_string();
        let content_type = field.content_type().map(|ct| ct.to_string());
        let filename = field.content_disposition().and_then(|cd| cd.get_filename().map(|s| s.to_string()));

        let mut bytes = Vec::new();
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => bytes.extend_from_slice(&data),
                Err(e) => {
                    return HttpResponse::BadRequest().json(ApiError::new(&format!("Upload error: {}", e)));
                }
            }
        }

        if name == "photo" {
            if !bytes.is_empty() {
                photo_data = Some((content_type, filename, bytes));
            }
        } else if name == "remove_photo" {
            if let Ok(s) = String::from_utf8(bytes) {
                remove_photo = s.trim() == "1" || s.eq_ignore_ascii_case("true");
            }
        } else if name == "roles[]" {
            if let Ok(s) = String::from_utf8(bytes) {
                if let Ok(id) = s.trim().parse::<i64>() {
                    roles.push(id);
                }
            }
        } else if name == "roles" {
            if let Ok(s) = String::from_utf8(bytes) {
                if let Some(parsed) = parse_roles(&s) {
                    roles.extend(parsed);
                }
            }
        } else if let Ok(text) = String::from_utf8(bytes) {
            fields.insert(name, text);
        }
    }

    if roles.is_empty() {
        if let Some(raw) = fields.get("roles") {
            if let Some(parsed) = parse_roles(raw) {
                roles = parsed;
            }
        }
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut sets = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    macro_rules! set_field {
        ($key:expr, $col:expr) => {
            if let Some(val) = fields.get($key) {
                sets.push(format!("{} = ?{}", $col, idx));
                params.push(Box::new(val.clone()));
                idx += 1;
            }
        };
    }

    set_field!("name", "name");
    set_field!("email", "email");
    set_field!("phone", "phone");
    if fields.contains_key("employee_id") {
        let employee_id = normalize_string_field(fields.get("employee_id").map(|s| s.as_str()).unwrap_or(""));
        if let Some(ref eid) = employee_id {
            if employee_id_taken(&conn, eid, Some(user_id)) {
                return HttpResponse::BadRequest()
                    .json(ApiError::new("Employee ID is already assigned to another user"));
            }
        }
        sets.push(format!("employee_id = ?{}", idx));
        params.push(Box::new(employee_id));
        idx += 1;
    }
    set_field!("status", "status");
    set_field!("work_location", "work_location");
    set_field!("date_of_joining", "date_of_joining");
    set_field!("account_number", "account_number");
    set_field!("ifsc_code", "ifsc_code");
    set_field!("bank_name", "bank_name");
    set_field!("account_type", "account_type");
    set_field!("pan_number", "pan_number");
    set_field!("esi_number", "esi_number");
    set_field!("pf_number", "pf_number");
    set_field!("aadhar_number", "aadhar_number");
    set_field!("date_of_exit", "date_of_exit");

    if let Some(v) = fields.get("department_id").and_then(|s| opt_i64(s)) {
        sets.push(format!("department_id = ?{}", idx));
        params.push(Box::new(v));
        idx += 1;
    }
    if let Some(v) = fields.get("designation_id").and_then(|s| opt_i64(s)) {
        sets.push(format!("designation_id = ?{}", idx));
        params.push(Box::new(v));
        idx += 1;
    }

    if remove_photo {
        if let Ok(old) = conn.query_row::<Option<String>, _, _>(
            "SELECT photo FROM users WHERE id=?1",
            [user_id],
            |r| r.get(0),
        ) {
            if let Some(ref p) = old {
                storage::delete_photo_path(p);
            }
        }
        sets.push(format!("photo = ?{}", idx));
        params.push(Box::new(None::<String>));
        idx += 1;
    } else if let Some((mime, fname, data)) = photo_data {
        match storage::save_user_photo(&data, mime.as_deref(), fname.as_deref()) {
            Ok(path) => {
                if let Ok(old) = conn.query_row::<Option<String>, _, _>(
                    "SELECT photo FROM users WHERE id=?1",
                    [user_id],
                    |r| r.get(0),
                ) {
                    if let Some(ref p) = old {
                        storage::delete_photo_path(p);
                    }
                }
                sets.push(format!("photo = ?{}", idx));
                params.push(Box::new(path));
                idx += 1;
            }
            Err(e) => return HttpResponse::BadRequest().json(ApiError::new(&e)),
        }
    }

    if sets.is_empty() && roles.is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("No fields to update"));
    }

    if !sets.is_empty() {
        sets.push(format!("updated_at = ?{}", idx));
        let now_for_roles = now.clone();
        params.push(Box::new(now.clone()));
        idx += 1;
        params.push(Box::new(user_id));

        let sql = format!("UPDATE users SET {} WHERE id = ?{}", sets.join(", "), idx);
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        if let Err(e) = conn.execute(&sql, param_refs.as_slice()) {
            return HttpResponse::BadRequest().json(ApiError::new(&format!("Failed to update user: {}", e)));
        }

        if !roles.is_empty() {
            let _ = conn.execute("DELETE FROM role_user WHERE user_id = ?1", [user_id]);
            for role_id in &roles {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO role_user (user_id, role_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![user_id, role_id, &now_for_roles, &now_for_roles],
                );
            }
        }
    } else if !roles.is_empty() {
        let _ = conn.execute("DELETE FROM role_user WHERE user_id = ?1", [user_id]);
        for role_id in &roles {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO role_user (user_id, role_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![user_id, role_id, &now, &now],
            );
        }
    }

    match load_user_summary(&conn, user_id) {
        Some(summary) => HttpResponse::Ok().json(ApiResponse::success(summary)),
        None => HttpResponse::NotFound().json(ApiError::new("User not found")),
    }
}

/// DELETE /api/admin/users/{id}
pub async fn destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let user_id = path.into_inner();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Soft delete
    match conn.execute(
        "UPDATE users SET deleted_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, user_id],
    ) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "User deleted successfully"
        }))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed to delete user: {}", e))),
    }
}

/// GET /api/admin/users/stats
pub async fn stats(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let total: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL", [], |r| r.get(0)).unwrap_or(0);
    let active: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND status = 'active'", [], |r| r.get(0)).unwrap_or(0);
    let on_leave: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND status = 'on-leave'", [], |r| r.get(0)).unwrap_or(0);

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "total": total,
        "active": active,
        "on_leave": on_leave,
        "inactive": total - active - on_leave,
    })))
}

/// GET /api/admin/users/list (simple list for dropdowns)
pub async fn list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let mut stmt = conn.prepare(
        "SELECT id, name, email, employee_id FROM users WHERE deleted_at IS NULL ORDER BY name"
    ).unwrap();

    let users: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "email": row.get::<_, String>(2)?,
            "employee_id": row.get::<_, Option<String>>(3)?,
        }))
    }).unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(ApiResponse::success(users))
}
