use actix_web::{web, HttpRequest, HttpResponse};
use crate::db::DbPool; use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse}; use crate::models::career::Career;

pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let mut stmt = conn.prepare("SELECT * FROM careers ORDER BY created_at DESC").unwrap();
    let items: Vec<Career> = stmt.query_map([], Career::from_row).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}
pub async fn show(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    match conn.query_row("SELECT * FROM careers WHERE id=?1", [path.into_inner()], Career::from_row) {
        Ok(c)=>HttpResponse::Ok().json(ApiResponse::success(c)), Err(_)=>HttpResponse::NotFound().json(ApiError::new("Not found"))
    }
}
pub async fn store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<crate::models::career::CreateCareerRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match conn.execute("INSERT INTO careers (title,department,location,employment_type,description,requirements,salary_range,is_active,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,1,?8,?9)",
        rusqlite::params![body.title, body.department, body.location, body.employment_type, body.description, body.requirements, body.salary_range, &now, &now]) {
        Ok(_)=>HttpResponse::Created().json(ApiResponse::success(serde_json::json!({"id": conn.last_insert_rowid()}))),
        Err(e)=>HttpResponse::BadRequest().json(ApiError::new(&format!("{}",e)))
    }
}
pub async fn update(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>, body: web::Json<crate::models::career::CreateCareerRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = conn.execute("UPDATE careers SET title=?1,department=?2,location=?3,employment_type=?4,description=?5,requirements=?6,salary_range=?7,updated_at=?8 WHERE id=?9",
        rusqlite::params![body.title, body.department, body.location, body.employment_type, body.description, body.requirements, body.salary_range, &now, path.into_inner()]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Updated"})))
}
pub async fn destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let id = path.into_inner();
    let app_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM job_applications WHERE career_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if app_count > 0 {
        let _ = conn.execute(
            "UPDATE careers SET is_active=0, updated_at=?1 WHERE id=?2",
            rusqlite::params![&now, id],
        );
        return HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "Career deactivated (applications preserved)"
        })));
    }
    let _ = conn.execute("DELETE FROM careers WHERE id=?1", [id]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Deleted"})))
}
pub async fn stats(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let t: i64 = conn.query_row("SELECT COUNT(*) FROM careers", [], |r| r.get(0)).unwrap_or(0);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"total": t})))
}
pub async fn list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse { index(pool, req).await }

/// GET /api/public/careers — active job postings (no auth)
pub async fn public_list(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let mut stmt = match conn.prepare(
        "SELECT id, title, department, location, employment_type, description, requirements, salary_range, created_at
         FROM careers WHERE is_active = 1 ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(ApiError::new(&format!("{e}"))),
    };
    let items: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "title": row.get::<_, String>(1)?,
                "department": row.get::<_, Option<String>>(2)?,
                "location": row.get::<_, Option<String>>(3)?,
                "employment_type": row.get::<_, Option<String>>(4)?,
                "description": row.get::<_, Option<String>>(5)?,
                "requirements": row.get::<_, Option<String>>(6)?,
                "salary_range": row.get::<_, Option<String>>(7)?,
                "created_at": row.get::<_, Option<String>>(8)?,
            }))
        })
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

#[derive(Debug, serde::Deserialize)]
pub struct PublicApplyRequest {
    pub career_id: i64,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub cover_letter: Option<String>,
    pub resume_url: Option<String>,
}

/// POST /api/public/careers/apply — submit job application (no auth)
pub async fn public_apply(pool: web::Data<DbPool>, body: web::Json<PublicApplyRequest>) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let active: bool = conn
        .query_row(
            "SELECT is_active FROM careers WHERE id=?1",
            [body.career_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false);
    if !active {
        return HttpResponse::BadRequest().json(ApiError::new("Job posting is not active"));
    }
    let title: String = conn
        .query_row(
            "SELECT title FROM careers WHERE id=?1",
            [body.career_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "Open Position".into());

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let tracking = format!("APP-{}", chrono::Utc::now().timestamp());
    match conn.execute(
        "INSERT INTO job_applications (career_id, name, email, phone, cover_letter, resume, applied_position, status, tracking_number, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9, ?9)",
        rusqlite::params![
            body.career_id,
            body.name.trim(),
            body.email.trim(),
            body.phone,
            body.cover_letter,
            body.resume_url,
            title,
            tracking,
            &now,
        ],
    ) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
            "id": conn.last_insert_rowid(),
            "tracking_number": tracking,
            "message": "Application submitted successfully",
        }))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed: {e}"))),
    }
}
