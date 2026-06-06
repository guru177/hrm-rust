use actix_web::{web, HttpRequest, HttpResponse};
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Center {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Center {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            code: row.get("code")?,
            address: row.get("address")?,
            city: row.get("city")?,
            state: row.get("state")?,
            country: row.get("country")?,
            is_active: row.get::<_, Option<bool>>("is_active")?.unwrap_or(true),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateCenterRequest {
    pub name: String,
    pub code: Option<String>,
    pub address: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub place: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub pincode: Option<String>,
}

impl CreateCenterRequest {
    fn resolved_address(&self) -> Option<String> {
        if let Some(ref a) = self.address {
            if !a.is_empty() {
                return Some(a.clone());
            }
        }
        let parts: Vec<String> = [
            self.address_line1.clone(),
            self.address_line2.clone(),
            self.place.clone(),
            self.pincode.clone(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }
}

pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let mut stmt = match conn.prepare("SELECT * FROM centers ORDER BY name") {
        Ok(s) => s,
        Err(_) => return HttpResponse::Ok().json(ApiResponse::success(Vec::<Center>::new()))
    };
    let items: Vec<Center> = stmt.query_map([], Center::from_row).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

pub async fn store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<CreateCenterRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match conn.execute(
        "INSERT INTO centers (name,code,address,city,state,country,is_active,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8)",
        rusqlite::params![body.name, body.code, body.resolved_address(), body.city, body.state, body.country, &now, &now]
    ) {
        Ok(_)=>HttpResponse::Created().json(ApiResponse::success(serde_json::json!({"id": conn.last_insert_rowid()}))),
        Err(e)=>HttpResponse::BadRequest().json(ApiError::new(&format!("{}",e)))
    }
}

pub async fn update(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>, body: web::Json<CreateCenterRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = conn.execute(
        "UPDATE centers SET name=?1,code=?2,address=?3,city=?4,state=?5,country=?6,updated_at=?7 WHERE id=?8",
        rusqlite::params![body.name, body.code, body.resolved_address(), body.city, body.state, body.country, &now, path.into_inner()]
    );
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Updated"})))
}

pub async fn destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let _ = conn.execute("DELETE FROM centers WHERE id=?1", [path.into_inner()]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Deleted"})))
}
