use actix_web::{web, HttpRequest, HttpResponse};
use crate::db::DbPool; use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse}; use crate::models::workflow::Workflow;

pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let mut stmt = conn.prepare("SELECT * FROM workflows ORDER BY created_at DESC").unwrap();
    let items: Vec<Workflow> = stmt.query_map([], Workflow::from_row).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}
pub async fn show(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    match conn.query_row("SELECT * FROM workflows WHERE id=?1", [path.into_inner()], Workflow::from_row) {
        Ok(w)=>HttpResponse::Ok().json(ApiResponse::success(w)), Err(_)=>HttpResponse::NotFound().json(ApiError::new("Not found"))
    }
}
pub async fn store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<crate::models::workflow::CreateWorkflowRequest>) -> HttpResponse {
    let c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let actions_str = body.actions.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
    let trigger_conditions_str = body.trigger_conditions.as_ref().map(|v| v.to_string());
    let is_active = body.is_active.unwrap_or(true);
    let user_id = c.sub;
    match conn.execute("INSERT INTO workflows (name,description,trigger_type,trigger_conditions,actions,is_active,execution_count,created_by,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,0,?7,?8,?9)",
        rusqlite::params![body.name, body.description, body.trigger_type, trigger_conditions_str, actions_str, is_active, user_id, &now, &now]) {
        Ok(_)=>HttpResponse::Created().json(ApiResponse::success(serde_json::json!({"id": conn.last_insert_rowid()}))),
        Err(e)=>HttpResponse::BadRequest().json(ApiError::new(&format!("{}",e)))
    }
}
pub async fn update(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>, body: web::Json<crate::models::workflow::CreateWorkflowRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let actions_str = body.actions.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
    let trigger_conditions_str = body.trigger_conditions.as_ref().map(|v| v.to_string());
    let is_active = body.is_active.unwrap_or(true);
    let _ = conn.execute("UPDATE workflows SET name=?1,description=?2,trigger_type=?3,trigger_conditions=?4,actions=?5,is_active=?6,updated_at=?7 WHERE id=?8",
        rusqlite::params![body.name, body.description, body.trigger_type, trigger_conditions_str, actions_str, is_active, &now, path.into_inner()]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Updated"})))
}
pub async fn destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let _ = conn.execute("DELETE FROM workflows WHERE id=?1", [path.into_inner()]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Deleted"})))
}
pub async fn toggle(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let _ = conn.execute("UPDATE workflows SET is_active = NOT is_active WHERE id=?1", [path.into_inner()]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Toggled"})))
}

pub async fn duplicate(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let id = path.into_inner();
    let source = match conn.query_row("SELECT * FROM workflows WHERE id=?1", [id], Workflow::from_row) {
        Ok(w) => w,
        Err(_) => return HttpResponse::NotFound().json(ApiError::new("Not found")),
    };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let name = format!("{} (Copy)", source.name);
    let actions_str = source.actions.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
    let trigger_conditions_str = source.trigger_conditions.as_ref().map(|v| v.to_string());
    let is_active = source.is_active;
    match conn.execute(
        "INSERT INTO workflows (name,description,trigger_type,trigger_conditions,actions,is_active,execution_count,created_by,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,0,?7,?8,?9)",
        rusqlite::params![
            name,
            source.description,
            source.trigger_type,
            trigger_conditions_str,
            actions_str,
            is_active,
            c.sub,
            &now,
            &now,
        ],
    ) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(serde_json::json!({"id": conn.last_insert_rowid()}))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}
