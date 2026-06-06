use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};

const ATTENDANCE_PRESENT_SQL: &str =
    "SELECT COUNT(DISTINCT user_id) FROM attendance WHERE date=?1 AND deleted_at IS NULL AND clock_out IS NOT NULL";

pub async fn hr_dashboard(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let now = chrono::Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let month = now.month() as i32;
    let year = now.year();

    let total_employees: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE is_super_admin=0 AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let today_attendance: i64 = conn
        .query_row(ATTENDANCE_PRESENT_SQL, [&today], |r| r.get(0))
        .unwrap_or(0);
    let att_pct = if total_employees > 0 {
        (today_attendance as f64 / total_employees as f64 * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };
    let pending_requests: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE status='pending' AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let active_projects: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE status='in_progress'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let todo: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE status='todo'", [], |r| r.get(0))
        .unwrap_or(0);
    let in_progress: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status='in_progress'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let completed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status='completed'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let on_hold: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status='on_hold'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let leave_types: HashMap<String, i64> =
        crate::payroll_logic::approved_leave_days_by_type_in_month(&conn, month, year);
    let leave_types_json: serde_json::Map<String, serde_json::Value> = leave_types
        .into_iter()
        .map(|(k, v)| (k, serde_json::json!(v)))
        .collect();

    let mut trends = Vec::new();
    for i in (0..7).rev() {
        let d = now - chrono::Duration::days(i);
        let date_str = d.format("%Y-%m-%d").to_string();
        let count: i64 = conn
            .query_row(ATTENDANCE_PRESENT_SQL, [&date_str], |r| r.get(0))
            .unwrap_or(0);
        let pct = if total_employees > 0 {
            (count as f64 / total_employees as f64 * 100.0 * 10.0).round() / 10.0
        } else {
            0.0
        };
        trends.push(serde_json::json!({
            "date": d.format("%a").to_string(),
            "percentage": pct,
            "count": count,
        }));
    }

    let mut hstmt = conn
        .prepare("SELECT name, date FROM holidays WHERE date >= ?1 ORDER BY date LIMIT 4")
        .unwrap();
    let holidays: Vec<serde_json::Value> = hstmt
        .query_map([&today], |row| {
            let name: String = row.get(0)?;
            let date: String = row.get(1)?;
            let days_away = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .ok()
                .and_then(|hd| {
                    NaiveDate::parse_from_str(&today, "%Y-%m-%d")
                        .ok()
                        .map(|td| (hd - td).num_days())
                })
                .unwrap_or(0);
            Ok(serde_json::json!({
                "name": name,
                "date": date,
                "daysAway": days_away,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let current_month_gross: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(gross_salary),0) FROM payslips WHERE month=?1 AND year=?2 AND status='generated'",
            rusqlite::params![month, year],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let prev_month = if month == 1 { 12 } else { month - 1 };
    let prev_year = if month == 1 { year - 1 } else { year };
    let previous_month_gross: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(gross_salary),0) FROM payslips WHERE month=?1 AND year=?2 AND status='generated'",
            rusqlite::params![prev_month, prev_year],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let change = if previous_month_gross > 0.0 {
        ((current_month_gross - previous_month_gross) / previous_month_gross * 100.0).round()
    } else {
        0.0
    };

    let cal_days = crate::payroll_logic::calendar_days_in_month(month, year);
    let as_of = format!("{}-{:02}-{}", year, month, cal_days);
    let mut dept_stmt = match conn.prepare(
            "SELECT d.id, d.name, u.id AS user_id
             FROM departments d
             INNER JOIN users u ON u.department_id = d.id AND u.deleted_at IS NULL AND u.is_super_admin=0
             ORDER BY d.name",
        ) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("dashboard dept payroll query failed: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiError::new("Failed to load payroll by department"));
        }
    };
    let dept_rows: Vec<(i64, String, i64)> = dept_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut dept_map: std::collections::HashMap<i64, (String, i64, f64)> =
        std::collections::HashMap::new();
    for (dept_id, dept_name, user_id) in dept_rows {
        let gross = crate::salary_logic::monthly_gross_for_user(&conn, user_id, &as_of);
        let entry = dept_map
            .entry(dept_id)
            .or_insert((dept_name, 0, 0.0));
        entry.1 += 1;
        entry.2 += gross;
    }
    let mut by_department: Vec<serde_json::Value> = dept_map
        .into_values()
        .map(|(dept, employees, total_cost)| {
            let average = if employees > 0 {
                total_cost / employees as f64
            } else {
                0.0
            };
            serde_json::json!({
                "department": dept,
                "employees": employees,
                "totalCost": total_cost,
                "average": average,
            })
        })
        .collect();
    by_department.sort_by(|a, b| {
        b.get("totalCost")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .partial_cmp(&a.get("totalCost").and_then(|v| v.as_f64()).unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    by_department.truncate(8);

    let celebrations: Vec<serde_json::Value> = {
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT name, date_of_birth FROM users
             WHERE deleted_at IS NULL AND date_of_birth IS NOT NULL AND date_of_birth != ''
             LIMIT 20",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                let name: String = row.get(0)?;
                let dob: String = row.get(1)?;
                Ok((name, dob))
            }) {
                let today_date = NaiveDate::parse_from_str(&today, "%Y-%m-%d").ok();
                for row in rows.flatten() {
                    let (name, dob) = row;
                    let date_part = dob.split('T').next().or_else(|| dob.split(' ').next());
                    if let (Some(dp), Some(td)) = (date_part, today_date) {
                        if let Ok(parsed) = NaiveDate::parse_from_str(dp, "%Y-%m-%d") {
                            if let Some(this_year_bday) =
                                NaiveDate::from_ymd_opt(year, parsed.month(), parsed.day())
                            {
                                let days = (this_year_bday - td).num_days();
                                if days >= 0 && days <= 30 {
                                    result.push(serde_json::json!({
                                        "name": name,
                                        "type": "birthday",
                                        "date": this_year_bday.format("%Y-%m-%d").to_string(),
                                        "isSoon": days <= 7,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
        result
    };

    let recent_workflows: Vec<serde_json::Value> = {
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT we.id, w.name, we.status, we.trigger_type, we.updated_at
             FROM workflow_executions we
             JOIN workflows w ON w.id = we.workflow_id
             ORDER BY we.updated_at DESC LIMIT 5",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?.to_string(),
                    "process": row.get::<_, String>(1)?,
                    "status": row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "pending".to_string()),
                    "step": row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    "timestamp": row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                }))
            }) {
                for row in rows.flatten() {
                    result.push(row);
                }
            }
        }
        result
    };

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "metrics": {
            "totalEmployees": total_employees,
            "attendancePercentage": att_pct,
            "attendanceCount": today_attendance,
            "pendingRequests": pending_requests,
            "activeProjects": active_projects,
        },
        "attendance": {
            "leaveTypes": leave_types_json,
            "trends": trends,
            "upcomingHolidays": holidays,
        },
        "payroll": {
            "currentMonth": current_month_gross,
            "previousMonth": previous_month_gross,
            "change": change,
            "byDepartment": by_department,
        },
        "operations": {
            "taskProgress": {
                "todo": todo,
                "in_progress": in_progress,
                "completed": completed,
                "on_hold": on_hold,
            },
            "celebrations": celebrations,
            "recentWorkflows": recent_workflows,
        }
    })))
}
