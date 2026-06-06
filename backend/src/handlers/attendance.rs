use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Datelike, Local};
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};
use crate::models::attendance::{Attendance, AttendanceListQuery, ClockInRequest};
use crate::attendance_logic::{close_open_session_before_clock_in, combine_clock_out_datetime, combine_datetime, find_open_attendance_session};
use crate::shift_logic::{
    calc_duration_minutes, is_early_departure, is_late_arrival,
    resolve_shift_for_user, ShiftConfig,
};

fn session_json(att: &Attendance, shift: Option<&ShiftConfig>) -> serde_json::Value {
    let clock_in = att.clock_in.as_ref().map(|t| combine_datetime(&att.date, t));
    let clock_out = att.clock_out.as_ref().map(|t| {
        combine_clock_out_datetime(
            &att.date,
            att.clock_in.as_deref().unwrap_or("00:00:00"),
            t,
        )
    });
    serde_json::json!({
        "id": att.id,
        "user_id": att.user_id,
        "date": att.date,
        "clock_in": clock_in,
        "clock_out": clock_out,
        "duration_minutes": att.duration_minutes,
        "is_late": att.is_late,
        "is_early_exit": att.is_early_exit,
        "status": att.status,
        "source": att.source,
        "clock_in_face_verified": att.clock_in_face_verified,
        "clock_in_face_match_score": att.clock_in_face_match_score,
        "shift": shift.map(|s| s.to_json()).unwrap_or(serde_json::Value::Null),
    })
}

fn fetch_user_sessions(
    conn: &rusqlite::Connection,
    user_id: i64,
    date: &str,
) -> Vec<Attendance> {
    let sql = "SELECT * FROM attendance WHERE user_id=?1 AND date=?2 AND deleted_at IS NULL ORDER BY id DESC";
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map(rusqlite::params![user_id, date], Attendance::from_row);
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    list(pool, req, web::Query(AttendanceListQuery {
        search: None,
        status: None,
        page: Some(1),
        per_page: Some(100),
    })).await
}

pub async fn today(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let today = Local::now().format("%Y-%m-%d").to_string();
    let shift = resolve_shift_for_user(&conn, claims.sub, &today);
    let sessions = fetch_user_sessions(&conn, claims.sub, &today);
    let active_clock_in = sessions
        .iter()
        .find(|s| s.clock_out.is_none())
        .map(|s| session_json(s, Some(&shift)));
    let attendances: Vec<serde_json::Value> = sessions
        .iter()
        .map(|s| session_json(s, Some(&shift)))
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "active_clock_in": active_clock_in,
        "attendances": attendances,
        "total_sessions": sessions.len(),
        "shift": shift.to_json(),
    })))
}

pub async fn clock_in(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<ClockInRequest>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let now = Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let time = now.format("%H:%M:%S").to_string();
    let ts = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let shift = resolve_shift_for_user(&conn, claims.sub, &date);
    let is_late = is_late_arrival(&time, &shift.start_time, &shift.end_time, shift.grace_in_minutes);
    let face_verified = body.face_verified.unwrap_or(false);
    let location_json = body.location.as_ref().and_then(|loc| serde_json::to_string(loc).ok());

    // Close any open session (today or prior-day overnight) before starting a new one
    close_open_session_before_clock_in(&conn, claims.sub, &date, &time, &ts, &shift);

    match conn.execute(
        "INSERT INTO attendance (user_id, date, clock_in, status, is_late, clock_in_location, clock_in_face_verified, clock_in_face_match_score, source, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'present', ?4, ?5, ?6, ?7, 'manual', ?8, ?8)",
        rusqlite::params![
            claims.sub,
            &date,
            &time,
            if is_late { 1 } else { 0 },
            location_json,
            if face_verified { 1 } else { 0 },
            body.face_match_score,
            &ts,
        ],
    ) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "Clocked in",
            "time": time,
            "shift": shift.to_json(),
            "is_late": is_late,
        }))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}

pub async fn clock_out(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let now = Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let time = now.format("%H:%M:%S").to_string();
    let ts = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let shift = resolve_shift_for_user(&conn, claims.sub, &date);

    let active = find_open_attendance_session(&conn, claims.sub, &date);

    let Some((att_id, session_date, clock_in)) = active else {
        return HttpResponse::BadRequest().json(ApiError::new("No active clock-in session found"));
    };

    let session_shift = resolve_shift_for_user(&conn, claims.sub, &session_date);
    let duration = calc_duration_minutes(&clock_in, &time);
    let early_exit = is_early_departure(
        &time,
        &session_shift.start_time,
        &session_shift.end_time,
        session_shift.grace_out_minutes,
    );

    match conn.execute(
        "UPDATE attendance SET clock_out=?1, duration_minutes=?2, is_early_exit=?3, updated_at=?4 WHERE id=?5",
        rusqlite::params![&time, duration, if early_exit { 1 } else { 0 }, &ts, att_id],
    ) {
        Ok(rows) if rows > 0 => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "Clocked out",
            "time": time,
            "duration_minutes": duration,
            "shift": shift.to_json(),
            "is_early_exit": early_exit,
        }))),
        Ok(_) => HttpResponse::BadRequest().json(ApiError::new("Could not clock out")),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}

pub async fn list(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<AttendanceListQuery>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let mut conditions = vec!["a.deleted_at IS NULL".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if !claims.is_super_admin {
        conditions.push("a.user_id = ?".to_string());
        params.push(Box::new(claims.sub));
    }

    if let Some(ref status) = query.status {
        if !status.is_empty() && status != "all" {
            conditions.push("a.status = ?".to_string());
            params.push(Box::new(status.clone()));
        }
    }

    if let Some(ref search) = query.search {
        if !search.is_empty() {
            conditions.push("(u.name LIKE ? OR u.email LIKE ?)".to_string());
            let like = format!("%{}%", search);
            params.push(Box::new(like.clone()));
            params.push(Box::new(like));
        }
    }

    let where_clause = conditions.join(" AND ");
    let count_sql = format!(
        "SELECT COUNT(*) FROM attendance a LEFT JOIN users u ON u.id = a.user_id WHERE {}",
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
        "SELECT a.*, u.name as user_name, u.email as user_email
         FROM attendance a
         LEFT JOIN users u ON u.id = a.user_id
         WHERE {}
         ORDER BY a.date DESC, a.id DESC
         LIMIT ? OFFSET ?",
        where_clause
    );

    let mut list_params: Vec<Box<dyn rusqlite::types::ToSql>> = params;
    list_params.push(Box::new(per_page));
    list_params.push(Box::new(offset));

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(ApiError::new(&format!("{}", e))),
    };

    struct ListRow {
        att: Attendance,
        user_name: Option<String>,
        user_email: Option<String>,
    }

    let list_rows: Vec<ListRow> = stmt
        .query_map(
            rusqlite::params_from_iter(list_params.iter().map(|p| p.as_ref())),
            |row| {
                Ok(ListRow {
                    att: Attendance::from_row(row)?,
                    user_name: row.get("user_name").ok(),
                    user_email: row.get("user_email").ok(),
                })
            },
        )
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let rows: Vec<serde_json::Value> = list_rows
        .iter()
        .map(|item| {
            let shift = resolve_shift_for_user(&conn, item.att.user_id, &item.att.date);
            let mut session = session_json(&item.att, Some(&shift));
            if let Some(obj) = session.as_object_mut() {
                obj.insert(
                    "user".to_string(),
                    serde_json::json!({
                        "id": item.att.user_id,
                        "name": item.user_name,
                        "email": item.user_email,
                    }),
                );
            }
            session
        })
        .collect();

    let last_page = ((total as f64) / (per_page as f64)).ceil().max(1.0) as i64;
    let from = if total > 0 { offset + 1 } else { 0 };
    let to = (offset + rows.len() as i64).min(total);

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "data": rows,
        "current_page": page,
        "last_page": last_page,
        "total": total,
        "from": from,
        "to": to,
        "per_page": per_page,
    })))
}

pub async fn users(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let mut stmt = match conn.prepare(
        "SELECT id, name, email FROM users WHERE deleted_at IS NULL ORDER BY name",
    ) {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let items: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "email": row.get::<_, Option<String>>(2)?,
            }))
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    HttpResponse::Ok().json(ApiResponse::success(items))
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

    let today = Local::now();
    let month_start = format!("{}-{:02}-01", today.year(), today.month());

    let user_clause = if claims.is_super_admin {
        ""
    } else {
        " AND user_id=?2"
    };

    let month_params: Vec<Box<dyn rusqlite::types::ToSql>> = if claims.is_super_admin {
        vec![Box::new(month_start.clone())]
    } else {
        vec![Box::new(month_start.clone()), Box::new(claims.sub)]
    };

    let q = |sql: &str| -> i64 {
        conn.query_row(
            &format!("{sql}{user_clause}"),
            rusqlite::params_from_iter(month_params.iter().map(|p| p.as_ref())),
            |r| r.get(0),
        )
        .unwrap_or(0)
    };

    let total_days = q("SELECT COUNT(DISTINCT date) FROM attendance WHERE deleted_at IS NULL AND date >= ?1");
    let present_days = q("SELECT COUNT(*) FROM attendance WHERE deleted_at IS NULL AND clock_out IS NOT NULL AND date >= ?1");
    let late_days = q("SELECT COUNT(*) FROM attendance WHERE deleted_at IS NULL AND is_late=1 AND date >= ?1");
    let early_exit_days = q("SELECT COUNT(*) FROM attendance WHERE deleted_at IS NULL AND is_early_exit=1 AND date >= ?1");
    let total_minutes = q("SELECT COALESCE(SUM(duration_minutes),0) FROM attendance WHERE deleted_at IS NULL AND date >= ?1");

    if claims.is_super_admin {
        let today_str = today.format("%Y-%m-%d").to_string();
        let present: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT user_id) FROM attendance WHERE date=?1 AND deleted_at IS NULL AND clock_out IS NOT NULL",
                [&today_str],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND is_super_admin=0",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        return HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "total_days": total_days,
            "present_days": present_days,
            "absent_days": (total - present).max(0),
            "late_days": late_days,
            "early_exit_days": early_exit_days,
            "total_hours": total_minutes / 60,
            "present": present,
            "total": total,
            "absent": (total - present).max(0),
        })));
    }

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "total_days": total_days,
        "present_days": present_days,
        "absent_days": 0,
        "late_days": late_days,
        "early_exit_days": early_exit_days,
        "total_hours": total_minutes / 60,
    })))
}
