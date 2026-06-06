use actix_web::{web, HttpRequest, HttpResponse};
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};
use crate::models::designation::{Designation, CreateDesignationRequest};

pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")) };
    let mut stmt = conn.prepare("SELECT * FROM designations ORDER BY name").unwrap();
    let items: Vec<Designation> = stmt.query_map([], Designation::from_row).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

pub async fn show(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")) };
    match conn.query_row("SELECT * FROM designations WHERE id = ?1", [path.into_inner()], Designation::from_row) {
        Ok(d) => HttpResponse::Ok().json(ApiResponse::success(d)),
        Err(_) => HttpResponse::NotFound().json(ApiError::new("Designation not found")),
    }
}

pub async fn store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<CreateDesignationRequest>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")) };
    let slug = body.name.to_lowercase().replace(' ', "-");
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match conn.execute("INSERT INTO designations (name, slug, description, level, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![body.name, slug, body.description, body.level, &now, &now]) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(serde_json::json!({"id": conn.last_insert_rowid()}))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed: {}", e))),
    }
}

pub async fn update(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>, body: web::Json<CreateDesignationRequest>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")) };
    let slug = body.name.to_lowercase().replace(' ', "-");
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match conn.execute("UPDATE designations SET name=?1, slug=?2, description=?3, level=?4, updated_at=?5 WHERE id=?6",
        rusqlite::params![body.name, slug, body.description, body.level, &now, path.into_inner()]) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Updated"}))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed: {}", e))),
    }
}

pub async fn destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")) };
    match conn.execute("DELETE FROM designations WHERE id = ?1", [path.into_inner()]) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Deleted"}))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed: {}", e))),
    }
}

pub async fn stats(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")) };
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM designations", [], |r| r.get(0)).unwrap_or(0);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"total": total})))
}

pub async fn list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")) };
    let mut stmt = conn.prepare("SELECT id, name FROM designations ORDER BY name").unwrap();
    let items: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({"id": row.get::<_, i64>(0)?, "name": row.get::<_, String>(1)?}))
    }).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}
