use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::db::DbPool;
use crate::leave_type_logic::{load_all, payment_type_label, LeaveTypeConfig};
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};

fn type_json(t: &LeaveTypeConfig) -> serde_json::Value {
    serde_json::json!({
        "id": t.id,
        "slug": t.slug,
        "name": t.name,
        "payment_type": t.payment_type,
        "payment_type_label": payment_type_label(&t.payment_type),
        "counts_toward_quota": t.counts_toward_quota,
        "is_active": t.is_active,
    })
}

/// GET /api/admin/leave-types — active types for forms
pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let items: Vec<_> = crate::leave_type_logic::load_active(&conn)
        .iter()
        .map(type_json)
        .collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

/// GET /api/admin/settings/leave-types — all types for admin config
pub async fn settings_list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let items: Vec<_> = load_all(&conn).iter().map(type_json).collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

#[derive(Debug, Deserialize)]
pub struct StoreLeaveTypeRequest {
    pub name: String,
    pub slug: Option<String>,
    pub payment_type: String,
    pub counts_toward_quota: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLeaveTypeRequest {
    pub name: Option<String>,
    pub payment_type: Option<String>,
    pub counts_toward_quota: Option<bool>,
    pub is_active: Option<bool>,
}

fn normalize_payment_type(v: &str) -> Result<String, String> {
    match v {
        "paid" | "unpaid" | "half_day" => Ok(v.to_string()),
        _ => Err("payment_type must be paid, unpaid, or half_day".into()),
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// POST /api/admin/settings/leave-types
pub async fn store(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<StoreLeaveTypeRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    if body.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("Name is required"));
    }
    let payment_type = match normalize_payment_type(&body.payment_type) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(ApiError::new(&e)),
    };
    let slug = body
        .slug
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| slugify(&body.name));
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM leave_types WHERE slug=?1",
            [&slug],
            |_| Ok(()),
        )
        .is_ok();
    if exists {
        return HttpResponse::BadRequest().json(ApiError::new("Leave type slug already exists"));
    }
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let quota = if body.counts_toward_quota.unwrap_or(false) {
        1
    } else {
        0
    };
    let active = if body.is_active.unwrap_or(true) { 1 } else { 0 };
    match conn.execute(
        "INSERT INTO leave_types (slug, name, payment_type, counts_toward_quota, is_active, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?6)",
        rusqlite::params![slug, body.name.trim(), payment_type, quota, active, &now],
    ) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
            "id": conn.last_insert_rowid(),
        }))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}

/// PUT /api/admin/settings/leave-types/{id}
pub async fn update(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateLeaveTypeRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let current: Option<(String, String, i64, i64)> = conn
        .query_row(
            "SELECT name, payment_type, counts_toward_quota, is_active FROM leave_types WHERE id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok();
    let Some((cur_name, cur_payment, cur_quota, cur_active)) = current else {
        return HttpResponse::NotFound().json(ApiError::new("Leave type not found"));
    };
    let name = body.name.as_deref().unwrap_or(&cur_name).trim().to_string();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("Name is required"));
    }
    let payment_type = if let Some(ref pt) = body.payment_type {
        match normalize_payment_type(pt) {
            Ok(v) => v,
            Err(e) => return HttpResponse::BadRequest().json(ApiError::new(&e)),
        }
    } else {
        cur_payment
    };
    let quota = body
        .counts_toward_quota
        .map(|v| if v { 1 } else { 0 })
        .unwrap_or(cur_quota);
    let active = body
        .is_active
        .map(|v| if v { 1 } else { 0 })
        .unwrap_or(cur_active);
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    match conn.execute(
        "UPDATE leave_types SET name=?1, payment_type=?2, counts_toward_quota=?3, is_active=?4, updated_at=?5 WHERE id=?6",
        rusqlite::params![name, payment_type, quota, active, &now, id],
    ) {
        Ok(n) if n > 0 => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"updated": true}))),
        Ok(_) => HttpResponse::NotFound().json(ApiError::new("Leave type not found")),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}
