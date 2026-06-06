use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};

#[derive(Debug, Deserialize)]
pub struct LeaveListQuery {
    pub status: Option<String>,
    pub leave_type: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectLeaveRequest {
    pub rejection_reason: Option<String>,
}

fn leave_days_between(conn: &rusqlite::Connection, user_id: i64, start: &str, end: &str) -> i64 {
    let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok();
    let end_date = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok();
    match (start_date, end_date) {
        (Some(s), Some(e)) => crate::payroll_logic::working_days_between_for_user(conn, user_id, s, e),
        _ => 1,
    }
}

fn has_overlapping_leave(
    conn: &rusqlite::Connection,
    user_id: i64,
    start: &str,
    end: &str,
    exclude_id: Option<i64>,
) -> bool {
    let sql = if exclude_id.is_some() {
        "SELECT 1 FROM leave_requests WHERE user_id=?1 AND deleted_at IS NULL
         AND status NOT IN ('rejected') AND start_date <= ?3 AND end_date >= ?2 AND id != ?4 LIMIT 1"
    } else {
        "SELECT 1 FROM leave_requests WHERE user_id=?1 AND deleted_at IS NULL
         AND status NOT IN ('rejected') AND start_date <= ?3 AND end_date >= ?2 LIMIT 1"
    };
    if let Some(eid) = exclude_id {
        conn.query_row(sql, rusqlite::params![user_id, start, end, eid], |_| Ok(()))
            .is_ok()
    } else {
        conn.query_row(sql, rusqlite::params![user_id, start, end], |_| Ok(()))
            .is_ok()
    }
}

fn leave_to_json(row: &rusqlite::Row) -> rusqlite::Result<serde_json::Value> {
    let id: i64 = row.get("id")?;
    let user_id: i64 = row.get("user_id")?;
    Ok(serde_json::json!({
        "id": id,
        "user_id": user_id,
        "leave_type": row.get::<_, String>("leave_type")?,
        "start_date": row.get::<_, String>("start_date")?,
        "end_date": row.get::<_, String>("end_date")?,
        "days_count": row.get::<_, i64>("days_count").unwrap_or(1),
        "reason": row.get::<_, Option<String>>("reason")?,
        "status": row.get::<_, String>("status")?,
        "remarks": row.get::<_, Option<String>>("remarks").ok().flatten(),
        "rejection_reason": row.get::<_, Option<String>>("rejection_reason").ok().flatten(),
        "approved_by": row.get::<_, Option<i64>>("approved_by").ok().flatten(),
        "created_at": row.get::<_, Option<String>>("created_at").ok().flatten(),
        "updated_at": row.get::<_, Option<String>>("updated_at").ok().flatten(),
        "user": {
            "id": user_id,
            "name": row.get::<_, Option<String>>("user_name").ok().flatten(),
            "email": row.get::<_, Option<String>>("user_email").ok().flatten(),
        }
    }))
}

fn fetch_leave_list(
    conn: &rusqlite::Connection,
    query: &LeaveListQuery,
    user_id: Option<i64>,
) -> serde_json::Value {
    let per_page = query.per_page.unwrap_or(15).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let mut conditions = vec!["lr.deleted_at IS NULL".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(uid) = user_id {
        conditions.push("lr.user_id = ?".to_string());
        params.push(Box::new(uid));
    }

    if let Some(ref status) = query.status {
        if !status.is_empty() && status != "all" {
            conditions.push("lr.status = ?".to_string());
            params.push(Box::new(status.clone()));
        }
    }

    if let Some(ref leave_type) = query.leave_type {
        if !leave_type.is_empty() && leave_type != "all" {
            conditions.push("lr.leave_type = ?".to_string());
            params.push(Box::new(leave_type.clone()));
        }
    }

    if let Some(ref search) = query.search {
        if !search.is_empty() {
            conditions.push("(u.name LIKE ? OR u.email LIKE ? OR lr.reason LIKE ?)".to_string());
            let like = format!("%{}%", search);
            params.push(Box::new(like.clone()));
            params.push(Box::new(like.clone()));
            params.push(Box::new(like));
        }
    }

    let where_clause = conditions.join(" AND ");
    let sort_col = match query.sort_by.as_deref() {
        Some("start_date") => "lr.start_date",
        Some("status") => "lr.status",
        Some("leave_type") => "lr.leave_type",
        _ => "lr.created_at",
    };
    let sort_dir = if query.sort_order.as_deref() == Some("asc") {
        "ASC"
    } else {
        "DESC"
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM leave_requests lr LEFT JOIN users u ON u.id = lr.user_id WHERE {}",
        where_clause
    );
    let total: i64 = conn
        .query_row(
            &count_sql,
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |r| r.get(0),
        )
        .unwrap_or(0);

    let sql = format!(
        "SELECT lr.*, u.name as user_name, u.email as user_email
         FROM leave_requests lr
         LEFT JOIN users u ON u.id = lr.user_id
         WHERE {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        where_clause, sort_col, sort_dir
    );

    let mut list_params = params;
    list_params.push(Box::new(per_page));
    list_params.push(Box::new(offset));

    let mut stmt = conn.prepare(&sql).unwrap();
    let items: Vec<serde_json::Value> = stmt
        .query_map(
            rusqlite::params_from_iter(list_params.iter().map(|p| p.as_ref())),
            leave_to_json,
        )
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let last_page = ((total as f64) / (per_page as f64)).ceil().max(1.0) as i64;
    let from = if total > 0 { offset + 1 } else { 0 };
    let to = (offset + items.len() as i64).min(total);

    serde_json::json!({
        "data": items,
        "current_page": page,
        "last_page": last_page,
        "total": total,
        "from": from,
        "to": to,
        "per_page": per_page,
    })
}

fn user_leave_stats(conn: &rusqlite::Connection, user_id: i64) -> serde_json::Value {
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE user_id=?1 AND deleted_at IS NULL",
            [user_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE user_id=?1 AND status='pending' AND deleted_at IS NULL",
            [user_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let approved: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE user_id=?1 AND status='approved' AND deleted_at IS NULL",
            [user_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let total_leave_days: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(days_count),0) FROM leave_requests WHERE user_id=?1 AND status='approved' AND deleted_at IS NULL",
            [user_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    serde_json::json!({
        "total_requests": total,
        "pending": pending,
        "approved": approved,
        "total_leave_days": total_leave_days,
    })
}

pub async fn index(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<LeaveListQuery>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let data = fetch_leave_list(&conn, &query, Some(claims.sub));
    HttpResponse::Ok().json(ApiResponse::success(data))
}

pub async fn list(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<LeaveListQuery>,
) -> HttpResponse {
    index(pool, req, query).await
}

pub async fn store(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<crate::models::leave_request::CreateLeaveRequest>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    if body.end_date < body.start_date {
        return HttpResponse::BadRequest().json(ApiError::new("End date must be on or after start date"));
    }
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let days = leave_days_between(&conn, claims.sub, &body.start_date, &body.end_date);
    if days <= 0 {
        return HttpResponse::BadRequest().json(ApiError::new("Invalid leave date range"));
    }
    if has_overlapping_leave(&conn, claims.sub, &body.start_date, &body.end_date, None) {
        return HttpResponse::BadRequest().json(ApiError::new(
            "Leave dates overlap with an existing request",
        ));
    }
    if crate::leave_type_logic::counts_toward_quota(&conn, &body.leave_type)
        && crate::payroll_logic::would_exceed_annual_quota(
            &conn,
            claims.sub,
            &body.start_date,
            &body.end_date,
            &body.leave_type,
        )
    {
        let quota = crate::payroll_logic::annual_leave_quota(&conn);
        return HttpResponse::BadRequest().json(ApiError::new(&format!(
            "Annual leave balance exceeded (quota: {} business days)",
            quota
        )));
    }

    match conn.execute(
        "INSERT INTO leave_requests (user_id, leave_type, start_date, end_date, days_count, reason, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?7)",
        rusqlite::params![
            claims.sub,
            body.leave_type,
            body.start_date,
            body.end_date,
            days,
            body.reason,
            &now,
        ],
    ) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
            "id": conn.last_insert_rowid(),
        }))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}

pub async fn destroy(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let leave_id = path.into_inner();
    let leave_row: Option<(i64, String)> = conn
        .query_row(
            "SELECT user_id, status FROM leave_requests WHERE id=?1 AND deleted_at IS NULL",
            [leave_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let (owner, status) = match leave_row {
        Some(r) => r,
        None => return HttpResponse::NotFound().json(ApiError::new("Leave request not found")),
    };
    if status != "pending" {
        let perms = crate::middleware::rbac::load_user_permissions(&conn, claims.sub, false);
        if !claims.is_super_admin
            && !crate::middleware::rbac::has_permission(&perms, "manage-leave-requests")
        {
            return HttpResponse::Conflict().json(ApiError::new(
                "Only pending leave requests can be deleted",
            ));
        }
    }
    if owner != claims.sub && !claims.is_super_admin {
        let perms = crate::middleware::rbac::load_user_permissions(&conn, claims.sub, false);
        if !crate::middleware::rbac::has_permission(&perms, "manage-leave-requests") {
            return HttpResponse::Forbidden().json(ApiError::new("Not allowed to delete this leave request"));
        }
    }
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = conn.execute(
        "UPDATE leave_requests SET deleted_at=?1 WHERE id=?2",
        rusqlite::params![&now, leave_id],
    );
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Deleted"})))
}

pub async fn stats(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    HttpResponse::Ok().json(ApiResponse::success(user_leave_stats(&conn, claims.sub)))
}

pub async fn manage(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<LeaveListQuery>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let data = fetch_leave_list(&conn, &query, None);
    HttpResponse::Ok().json(ApiResponse::success(data))
}

pub async fn list_all(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<LeaveListQuery>,
) -> HttpResponse {
    manage(pool, req, query).await
}

pub async fn admin_stats(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE status='pending' AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let approved: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE status='approved' AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let rejected: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE status='rejected' AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "pending": pending,
        "approved": approved,
        "rejected": rejected,
        "total": total,
    })))
}

pub async fn approve(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let leave_id = path.into_inner();
    let leave_info: Option<(i64, String, String, String, i64, String)> = conn
        .query_row(
            "SELECT user_id, leave_type, start_date, end_date, days_count, status
             FROM leave_requests WHERE id=?1 AND deleted_at IS NULL",
            [leave_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .ok();

    let Some((user_id, leave_type, start_date, end_date, _days_count, status)) = leave_info else {
        return HttpResponse::NotFound().json(ApiError::new("Leave request not found"));
    };
    if status != "pending" {
        return HttpResponse::Conflict().json(ApiError::new(
            "Leave request not found or already processed",
        ));
    }
    if has_overlapping_leave(&conn, user_id, &start_date, &end_date, Some(leave_id)) {
        return HttpResponse::BadRequest().json(ApiError::new(
            "Leave dates overlap with an existing approved or pending request",
        ));
    }
    if crate::leave_type_logic::counts_toward_quota(&conn, &leave_type) {
        let year = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
            .map(|d| d.year())
            .unwrap_or_else(|_| chrono::Utc::now().year());
        if crate::payroll_logic::would_exceed_annual_quota(
            &conn,
            user_id,
            &start_date,
            &end_date,
            &leave_type,
        ) {
            let quota = crate::payroll_logic::annual_leave_quota(&conn);
            let used = crate::payroll_logic::employee_leave_used_in_year(&conn, user_id, year);
            return HttpResponse::BadRequest().json(ApiError::new(&format!(
                "Annual leave balance exceeded (used {} + requested > quota {})",
                used, quota
            )));
        }
    }

    let updated = conn.execute(
        "UPDATE leave_requests SET status='approved', approved_by=?1, approved_at=?2, updated_at=?2
         WHERE id=?3 AND status='pending' AND deleted_at IS NULL",
        rusqlite::params![claims.sub, &now, leave_id],
    );
    if updated.unwrap_or(0) == 0 {
        return HttpResponse::Conflict().json(ApiError::new(
            "Leave request not found or already processed",
        ));
    }

    crate::workflow_logic::trigger(
        &conn,
        "leave_approved",
        &serde_json::json!({
            "leave_id": leave_id,
            "user_id": user_id,
            "leave_type": leave_type,
            "approved_by": claims.sub,
        }),
    );

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Approved"})))
}

pub async fn reject(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<RejectLeaveRequest>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let reason = body.rejection_reason.clone().unwrap_or_default();
    let leave_id = path.into_inner();
    let updated = conn.execute(
        "UPDATE leave_requests SET status='rejected', approved_by=?1, rejection_reason=?2, updated_at=?3
         WHERE id=?4 AND status='pending' AND deleted_at IS NULL",
        rusqlite::params![claims.sub, reason, &now, leave_id],
    );
    if updated.unwrap_or(0) == 0 {
        return HttpResponse::Conflict().json(ApiError::new(
            "Leave request not found or already processed",
        ));
    }
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Rejected"})))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRemarksRequest {
    pub remarks: Option<String>,
}

pub async fn update_remarks(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateRemarksRequest>,
) -> HttpResponse {
    let _claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let leave_id = path.into_inner();
    let updated = conn.execute(
        "UPDATE leave_requests SET remarks=?1, updated_at=?2 WHERE id=?3 AND deleted_at IS NULL",
        rusqlite::params![body.remarks, &now, leave_id],
    );
    if updated.unwrap_or(0) == 0 {
        return HttpResponse::NotFound().json(ApiError::new("Leave request not found"));
    }
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Remarks updated"})))
}
