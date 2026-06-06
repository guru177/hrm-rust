use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use lettre::{Message, SmtpTransport, Transport};
use lettre::message::MultiPart;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use crate::db::DbPool; use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse}; use crate::models::job_application::JobApplication;

pub async fn index(pool: web::Data<DbPool>, req: HttpRequest, query: web::Query<crate::models::job_application::JobApplicationQuery>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    
    let mut sql = String::from("SELECT * FROM job_applications WHERE 1=1");
    let mut params: Vec<String> = Vec::new();

    if let Some(s) = &query.search {
        if !s.is_empty() {
            sql.push_str(" AND (name LIKE ? OR applied_position LIKE ?)");
            let like_s = format!("%{}%", s);
            params.push(like_s.clone());
            params.push(like_s);
        }
    }

    if let Some(exp) = &query.experience {
        if !exp.is_empty() && exp != "all" {
            match exp.as_str() {
                "0-2" => sql.push_str(" AND experience_years >= 0 AND experience_years <= 2"),
                "3-5" => sql.push_str(" AND experience_years >= 3 AND experience_years <= 5"),
                "6+" => sql.push_str(" AND experience_years >= 6"),
                _ => {}
            }
        }
    }

    if let Some(st) = &query.status {
        if !st.is_empty() && st != "all" {
            sql.push_str(" AND status = ?");
            params.push(st.clone());
        }
    }

    sql.push_str(" ORDER BY created_at DESC");

    let mut stmt = conn.prepare(&sql).unwrap();
    let items: Vec<JobApplication> = stmt.query_map(rusqlite::params_from_iter(params), JobApplication::from_row).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}
pub async fn show(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    match conn.query_row("SELECT * FROM job_applications WHERE id=?1", [path.into_inner()], JobApplication::from_row) {
        Ok(j)=>HttpResponse::Ok().json(ApiResponse::success(j)), Err(_)=>HttpResponse::NotFound().json(ApiError::new("Not found"))
    }
}
pub async fn store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<crate::models::job_application::CreateJobApplicationRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let tracking = format!("APP-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000").to_uppercase());
    match conn.execute(
        "INSERT INTO job_applications (career_id,name,email,phone,cover_letter,dob,applied_position,status,tracking_number,created_at,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,'pending',?8,?9,?9)",
        rusqlite::params![
            body.career_id,
            body.name,
            body.email,
            body.phone,
            body.cover_letter,
            body.date_of_birth,
            body.applied_position,
            tracking,
            &now,
        ],
    ) {
        Ok(_)=>HttpResponse::Created().json(ApiResponse::success(serde_json::json!({"id": conn.last_insert_rowid()}))),
        Err(e)=>HttpResponse::BadRequest().json(ApiError::new(&format!("{}",e)))
    }
}
pub async fn destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let _ = conn.execute("DELETE FROM job_applications WHERE id=?1", [path.into_inner()]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Deleted"})))
}
pub async fn stats(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let t: i64 = conn.query_row("SELECT COUNT(*) FROM job_applications", [], |r| r.get(0)).unwrap_or(0);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"total": t})))
}
pub async fn list(pool: web::Data<DbPool>, req: HttpRequest, query: web::Query<crate::models::job_application::JobApplicationQuery>) -> HttpResponse { index(pool, req, query).await }
const VALID_APP_STATUSES: &[&str] = &[
    "pending", "reviewing", "shortlisted", "interview", "offered", "hired", "rejected",
];

pub async fn update_status(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>, body: web::Json<crate::models::job_application::UpdateStatusRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    if !VALID_APP_STATUSES.contains(&body.status.as_str()) {
        return HttpResponse::BadRequest().json(ApiError::new("Invalid application status"));
    }
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = path.into_inner();
    let updated = conn.execute(
        "UPDATE job_applications SET status=?1,updated_at=?2 WHERE id=?3",
        rusqlite::params![body.status, &now, id],
    );
    if updated.unwrap_or(0) == 0 {
        return HttpResponse::NotFound().json(ApiError::new("Application not found"));
    }
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Status updated"})))
}

#[derive(Deserialize)]
pub struct IncomingResumeWebhook {
    pub candidate_name: String,
    pub candidate_email: String,
    pub candidate_phone: Option<String>,
    pub resume_url: String,
    pub email_body: Option<String>,
    pub position_hint: Option<String>,
}

pub async fn webhook_incoming_resume(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<IncomingResumeWebhook>,
) -> HttpResponse {
    let expected = std::env::var("WEBHOOK_SECRET").unwrap_or_default();
    if expected.is_empty() {
        return HttpResponse::ServiceUnavailable().json(ApiError::new(
            "Resume webhook is disabled — set WEBHOOK_SECRET to enable",
        ));
    }
    let provided = req
        .headers()
        .get("X-Webhook-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != expected {
        return HttpResponse::Unauthorized().json(ApiError::new("Invalid webhook secret"));
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("Database error")),
    };

    let applied_position = body.position_hint.clone().unwrap_or_else(|| "Open Position".to_string());
    let tracking_number = format!("APP-{}-{}", chrono::Utc::now().format("%Y"), chrono::Utc::now().timestamp());
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let career_id: i64 = conn.query_row("SELECT id FROM careers LIMIT 1", [], |row| row.get(0)).unwrap_or(1);

    let sql = "
        INSERT INTO job_applications (
            career_id, tracking_number, name, email, phone, resume, status,
            applied_position, source, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, 'webhook', ?8, ?8)
    ";

    let result = conn.execute(
        sql,
        rusqlite::params![
            career_id,
            tracking_number,
            body.candidate_name,
            body.candidate_email,
            body.candidate_phone.as_deref().unwrap_or(""),
            body.resume_url,
            applied_position,
            now,
        ],
    );

    match result {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "Resume ingested successfully",
            "tracking_number": tracking_number
        }))),
        Err(e) => {
            eprintln!("Webhook DB Error: {:?}", e);
            HttpResponse::InternalServerError().json(ApiError::new("Failed to save application"))
        }
    }
}

#[derive(Deserialize)]
pub struct SendEmailRequest {
    pub subject: String,
    pub body: String,
    pub html_body: Option<String>,
}

pub async fn send_email(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<SendEmailRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let application_id = path.into_inner();
    let email: String = match conn.query_row("SELECT email FROM job_applications WHERE id=?1", [application_id], |row| row.get(0)) {
        Ok(e) => e,
        Err(_) => return HttpResponse::NotFound().json(ApiError::new("Application not found")),
    };

    let get_config = |key: &str, default_env: &str| -> String {
        let db_val: Result<String, _> = conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        );
        match db_val {
            Ok(v) if !v.is_empty() => v,
            _ => std::env::var(default_env).unwrap_or_default(),
        }
    };

    let smtp_host = get_config("smtp_host", "SMTP_HOST");
    let smtp_user = get_config("smtp_user", "SMTP_USER");
    let smtp_pass = get_config("smtp_pass", "SMTP_PASS");
    let mut smtp_port = get_config("smtp_port", "SMTP_PORT");
    if smtp_port.is_empty() { smtp_port = "587".to_string(); }
    let mut smtp_from = get_config("smtp_from", "SMTP_FROM");
    if smtp_from.is_empty() { smtp_from = "no-reply@example.com".to_string(); }

    if smtp_host.is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("SMTP credentials not configured"));
    }

    let multipart = if let Some(html) = &body.html_body {
        MultiPart::alternative_plain_html(body.body.clone(), html.clone())
    } else {
        MultiPart::alternative_plain_html(body.body.clone(), format!("<p>{}</p>", body.body.replace("\n", "<br>")))
    };

    let email_message = match Message::builder()
        .from(smtp_from.parse().unwrap())
        .to(email.parse().unwrap())
        .subject(&body.subject)
        .multipart(multipart) {
            Ok(m) => m,
            Err(_) => return HttpResponse::BadRequest().json(ApiError::new("Failed to construct email")),
    };

    let creds = Credentials::new(smtp_user, smtp_pass);

    let mailer = SmtpTransport::starttls_relay(&smtp_host)
        .unwrap()
        .credentials(creds)
        .port(smtp_port.parse().unwrap_or(587))
        .build();

    let result = web::block(move || {
        mailer.send(&email_message)
    }).await;

    match result {
        Ok(Ok(_)) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Email sent successfully"}))),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(ApiError::new(&format!("SMTP send failed: {}", e))),
        Err(_) => HttpResponse::InternalServerError().json(ApiError::new("Failed to execute SMTP task")),
    }
}
