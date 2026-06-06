use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Datelike;
use serde::Deserialize;
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};
use crate::payroll_logic;

#[derive(Debug, Deserialize)]
pub struct PayrollMonthQuery {
    pub month: Option<i32>,
    pub year: Option<i32>,
    pub department_id: Option<i64>,
    pub center_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PayrollPreviewRequest {
    pub month: i32,
    pub year: i32,
    pub employee_ids: Vec<i64>,
    pub adjustments: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PayrollGenerateRequest {
    pub month: i32,
    pub year: i32,
    pub payslip_ids: Vec<i64>,
    pub common_adjustments: Option<Vec<serde_json::Value>>,
}

fn fetch_user_base(conn: &rusqlite::Connection, user_id: i64) -> Option<serde_json::Value> {
    conn.query_row(
        "SELECT u.id, u.name, u.email, u.photo, u.department_id, d.name, u.work_location
         FROM users u LEFT JOIN departments d ON d.id = u.department_id
         WHERE u.id=?1 AND u.deleted_at IS NULL",
        [user_id],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "email": row.get::<_, Option<String>>(2)?,
                "photo": row.get::<_, Option<String>>(3)?,
                "department_id": row.get::<_, Option<i64>>(4)?,
                "department_name": row.get::<_, Option<String>>(5)?,
                "work_location": row.get::<_, Option<String>>(6)?,
            }))
        },
    )
    .ok()
}

pub fn build_employee_payroll(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> Option<serde_json::Value> {
    let mut emp = fetch_user_base(conn, user_id)?;
    let working_days = payroll_logic::working_days_for_user(conn, user_id, month, year);
    let present_days = payroll_logic::employee_present_business_days(conn, user_id, month, year);
    let leave_days = payroll_logic::employee_leave_business_days(conn, user_id, month, year);
    let paid_holidays = payroll_logic::paid_holidays_for_user(conn, user_id, month, year);
    let cal_days = payroll_logic::calendar_days_in_month(month, year);
    let month_end = format!("{}-{:02}-{}", year, month, cal_days);

    let obj = emp.as_object_mut().unwrap();
    obj.insert("working_days".to_string(), serde_json::json!(working_days));
    obj.insert("calendar_days".to_string(), serde_json::json!(cal_days));
    obj.insert("present_days".to_string(), serde_json::json!(present_days));
    obj.insert("leave_days".to_string(), serde_json::json!(leave_days));
    obj.insert("paid_holidays".to_string(), serde_json::json!(paid_holidays));

    let Some(salary) = crate::salary_logic::load_user_salary(conn, user_id, &month_end) else {
        obj.insert("has_salary_structure".to_string(), serde_json::json!(false));
        obj.insert(
            "payroll_error".to_string(),
            serde_json::json!("No salary structure configured for this period"),
        );
        return Some(emp);
    };

    let gross = salary.gross;
    let lop_base = salary.lop_gross();
    let (lop, lop_breakdown) =
        payroll_logic::lop_amount_for_user_month(conn, user_id, month, year, working_days);
    let lop_days = lop_breakdown.days;

    let basic_after_lop = crate::salary_split::round2((salary.basic - lop_breakdown.basic).max(0.0));
    let gross_after_lop = lop_breakdown.net_after_lop;

    let comp = crate::salary_split::load_component_split_config(conn);
    let profile = crate::salary_split::load_employee_profile(conn, user_id, &month_end);
    let use_ctc_profile = profile.is_some();
    let use_component_deductions =
        use_ctc_profile || salary.source == "salary_structure_items";

    let pf_applicable = profile
        .as_ref()
        .map(|p| p.pf_applicable)
        .unwrap_or(true)
        && comp.has_pf;
    let esi_applicable = profile
        .as_ref()
        .map(|p| p.esi_applicable)
        .unwrap_or(true)
        && comp.has_esi;

    let statutory_cfg = crate::statutory_logic::load_statutory_config(conn);
    let advance = crate::statutory_logic::advance_emi_for_month(conn, user_id);

    let deduction_lines = if use_component_deductions {
        crate::salary_split::build_payroll_deduction_lines(
            &comp,
            &statutory_cfg,
            basic_after_lop,
            gross_after_lop,
            advance,
            pf_applicable,
            esi_applicable,
        )
    } else {
        Vec::new()
    };

    let statutory = if use_component_deductions {
        crate::salary_split::statutory_result_from_lines(&deduction_lines)
    } else {
        crate::statutory_logic::StatutoryResult {
            pf_employee: salary.pf,
            esi_employee: salary.esi,
            other_deductions: salary.tds + salary.other_deductions,
            total_employee: salary.fixed_deductions,
            ..Default::default()
        }
    };

    let total_deductions = if use_component_deductions {
        statutory.total_employee + lop
    } else {
        salary.fixed_deductions + lop
    };
    let net = (gross - total_deductions).max(0.0);

    let mut comp_list: Vec<serde_json::Value> = if use_component_deductions {
        salary
            .components
            .iter()
            .filter(|c| c.get("type").and_then(|v| v.as_str()) == Some("earning"))
            .cloned()
            .collect()
    } else {
        salary.components.clone()
    };

    for line in &lop_breakdown.lines {
        comp_list.push(serde_json::json!({
            "component_id": line.component_id,
            "name": line.name,
            "type": "deduction",
            "amount": line.amount,
        }));
    }
    if lop_breakdown.lines.is_empty() && lop > 0.0 {
        comp_list.push(serde_json::json!({"name": "LOP (Absent)", "type": "deduction", "amount": lop}));
    }

    if use_component_deductions {
        for line in deduction_lines {
            comp_list.push(serde_json::json!({
                "component_id": line.component_id,
                "name": line.name,
                "type": "deduction",
                "amount": line.amount,
            }));
        }
    }
    let shift_penalty = 0.0;

    let payroll_detail = serde_json::json!({
        "lop_breakdown": lop_breakdown,
        "statutory": statutory,
        "gross_after_lop": gross_after_lop,
        "basic_after_lop": basic_after_lop,
        "use_statutory": use_ctc_profile,
        "use_component_deductions": use_component_deductions,
        "source": salary.source,
    });

    obj.insert("has_salary_structure".to_string(), serde_json::json!(true));
    obj.insert("salary_source".to_string(), serde_json::json!(salary.source));
    obj.insert("absent_days".to_string(), serde_json::json!(lop_days));
    obj.insert("lop_days".to_string(), serde_json::json!(lop_days));
    obj.insert("shift_penalty".to_string(), serde_json::json!(0.0));
    obj.insert("gross_salary".to_string(), serde_json::json!(gross));
    obj.insert("gross_after_lop".to_string(), serde_json::json!(gross_after_lop));
    obj.insert("lop_gross".to_string(), serde_json::json!(lop_base));
    obj.insert("net_salary".to_string(), serde_json::json!(net));
    obj.insert("payroll_detail".to_string(), payroll_detail.clone());
    obj.insert(
        "salary_structure".to_string(),
        serde_json::json!({
            "gross_salary": gross,
            "gross_after_lop": gross_after_lop,
            "total_deductions": total_deductions,
            "lop_deduction": lop,
            "lop_breakdown": lop_breakdown,
            "shift_penalty": shift_penalty,
            "pf_deduction": if use_component_deductions { statutory.pf_employee } else { salary.pf },
            "esi_deduction": if use_component_deductions { statutory.esi_employee } else { salary.esi },
            "prof_tax": statutory.prof_tax,
            "advance_deduction": statutory.advance,
            "net_salary": net,
            "components": comp_list,
            "statutory": statutory,
        }),
    );

    if let Ok((pid, status)) = conn.query_row(
        "SELECT id, status FROM payslips WHERE user_id=?1 AND month=?2 AND year=?3",
        rusqlite::params![user_id, month, year],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    ) {
        obj.insert("payslip_id".to_string(), serde_json::json!(pid));
        obj.insert("payslip_status".to_string(), serde_json::json!(status));
    }

    Some(emp)
}

pub async fn index(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let mut stmt = conn
        .prepare("SELECT * FROM payslips ORDER BY year DESC, month DESC LIMIT 100")
        .unwrap();
    let items: Vec<crate::models::payslip::Payslip> = stmt
        .query_map([], crate::models::payslip::Payslip::from_row)
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

pub async fn list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    index(pool, req).await
}

pub async fn stats(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<PayrollMonthQuery>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let now = chrono::Utc::now();
    let month = query.month.unwrap_or(now.month() as i32);
    let year = query.year.unwrap_or(now.year());
    let paid_holidays = payroll_logic::total_paid_holidays_for_month(&conn, month, year);
    let cal_days = payroll_logic::calendar_days_in_month(month, year);
    let start = format!("{}-{:02}-01", year, month);
    let end = format!("{}-{:02}-{}", year, month, cal_days);

    let total_employees: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND is_super_admin=0",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let approved_leave_days =
        payroll_logic::approved_leave_business_days_in_month(&conn, month, year);
    let present_days_total: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT user_id || '-' || date) FROM attendance
             WHERE date >= ?1 AND date <= ?2 AND deleted_at IS NULL AND clock_out IS NOT NULL",
            rusqlite::params![start, end],
            |r| r.get(0),
        )
        .unwrap_or(0);

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "total_employees": total_employees,
        "approved_leaves": approved_leave_days,
        "present_days_total": present_days_total,
        "paid_holidays": paid_holidays,
        "total": conn.query_row(
            "SELECT COUNT(*) FROM payslips WHERE month=?1 AND year=?2 AND status='generated'",
            rusqlite::params![month, year], |r| r.get(0)
        ).unwrap_or(0),
        "total_gross": conn.query_row(
            "SELECT COALESCE(SUM(gross_salary),0) FROM payslips WHERE month=?1 AND year=?2 AND status='generated'",
            rusqlite::params![month, year], |r| r.get(0)
        ).unwrap_or(0.0),
    })))
}

pub async fn employees(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<PayrollMonthQuery>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let now = chrono::Utc::now();
    let month = query.month.unwrap_or(now.month() as i32);
    let year = query.year.unwrap_or(now.year());

    let mut sql = String::from(
        "SELECT u.id FROM users u WHERE u.deleted_at IS NULL AND u.is_super_admin=0",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(dept_id) = query.department_id {
        sql.push_str(" AND u.department_id=?");
        params.push(Box::new(dept_id));
    }
    if let Some(center_id) = query.center_id {
        sql.push_str(" AND u.work_location = (SELECT name FROM centers WHERE id=? LIMIT 1)");
        params.push(Box::new(center_id));
    }

    let mut stmt = conn.prepare(&sql).unwrap();
    let user_ids: Vec<i64> = stmt
        .query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let items: Vec<serde_json::Value> = user_ids
        .into_iter()
        .filter_map(|uid| build_employee_payroll(&conn, uid, month, year))
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(items))
}

pub async fn preview(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<PayrollPreviewRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let adj_map = payroll_logic::parse_employee_adjustments(&body.adjustments);
    let mut previews = Vec::new();

    for user_id in &body.employee_ids {
        let Some(emp) = build_employee_payroll(&conn, *user_id, body.month, body.year) else {
            continue;
        };

        if !emp
            .get("has_salary_structure")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            previews.push(serde_json::json!({
                "user_id": user_id,
                "user_name": emp.get("name").and_then(|v| v.as_str()).unwrap_or("Employee"),
                "skipped": true,
                "reason": emp.get("payroll_error").and_then(|v| v.as_str()).unwrap_or("No salary structure"),
            }));
            continue;
        }

        let gross = emp.get("gross_salary").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mut net = emp.get("net_salary").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mut total_deductions = emp
            .get("salary_structure")
            .and_then(|v| v.get("total_deductions"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let name = emp.get("name").and_then(|v| v.as_str()).unwrap_or("Employee");
        let working_days = emp.get("working_days").and_then(|v| v.as_i64()).unwrap_or(0);
        let present_days = emp.get("present_days").and_then(|v| v.as_i64()).unwrap_or(0);
        let leave_days = emp.get("leave_days").and_then(|v| v.as_i64()).unwrap_or(0);
        let paid_holidays = emp.get("paid_holidays").and_then(|v| v.as_i64()).unwrap_or(0);
        let lop = emp
            .get("salary_structure")
            .and_then(|v| v.get("lop_deduction"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let shift_penalty = emp.get("shift_penalty").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ss = emp.get("salary_structure");
        let lop_bd = ss.and_then(|v| v.get("lop_breakdown"));
        let lop_basic = lop_bd.and_then(|v| v.get("basic")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lop_hra = lop_bd.and_then(|v| v.get("hra")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lop_transport = lop_bd.and_then(|v| v.get("conveyance")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lop_other = lop_bd.and_then(|v| v.get("special")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let pf = ss.and_then(|v| v.get("pf_deduction")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let esi = ss.and_then(|v| v.get("esi_deduction")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let prof_tax = ss.and_then(|v| v.get("prof_tax")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let advance = ss.and_then(|v| v.get("advance_deduction")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lw_employee = ss
            .and_then(|v| v.get("statutory"))
            .and_then(|v| v.get("lw_employee"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let tds = ss
            .and_then(|v| v.get("statutory"))
            .and_then(|v| v.get("other_deductions"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let payroll_detail = emp.get("payroll_detail").map(|v| v.to_string()).unwrap_or_else(|| "{}".into());

        let cal_days = payroll_logic::calendar_days_in_month(body.month, body.year);
        let month_end = format!("{}-{:02}-{}", body.year, body.month, cal_days);
        let salary = crate::salary_logic::load_user_salary(&conn, *user_id, &month_end);
        let (basic, hra, transport, other) = if let Some(ref s) = salary {
            (
                crate::salary_split::round2((s.basic - lop_basic).max(0.0)),
                crate::salary_split::round2((s.hra - lop_hra).max(0.0)),
                crate::salary_split::round2((s.transport - lop_transport).max(0.0)),
                crate::salary_split::round2((s.other_earnings - lop_other).max(0.0)),
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        let user_adjs = adj_map.get(user_id).map(|v| v.as_slice()).unwrap_or(&[]);
        let (adj_net, adj_ded, adj_json) =
            payroll_logic::apply_adjustment_list(gross, net, total_deductions, user_adjs);
        net = adj_net;
        total_deductions = adj_ded;

        let existing: Option<(i64, String)> = conn
            .query_row(
                "SELECT id, status FROM payslips WHERE user_id=?1 AND month=?2 AND year=?3",
                rusqlite::params![user_id, body.month, body.year],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((pid, ref status)) = existing {
            if status == "generated" {
                previews.push(serde_json::json!({
                    "id": pid,
                    "user_id": user_id,
                    "user_name": name,
                    "skipped": true,
                    "reason": "Payslip already generated — unlock before re-previewing",
                }));
                continue;
            }
        }

        let payslip_id = if let Some((pid, _)) = existing {
            let _ = conn.execute(
                "UPDATE payslips SET working_days=?1, present_days=?2, leave_days=?3, holiday_days=?4,
                 basic_salary=?5, hra=?6, transport_allowance=?7, other_allowances=?8, gross_salary=?9,
                 lop_deduction=?10, lop_basic=?11, lop_hra=?12, lop_transport=?13, lop_other=?14,
                 shift_penalty=?15, pf_deduction=?16, esi_deduction=?17, tds=?18, prof_tax=?19,
                 advance_deduction=?20, lw_employee=?21, total_deductions=?22, net_salary=?23,
                 adjustments=?24, payroll_detail=?25, status='draft', updated_at=?26
                 WHERE id=?27 AND status='draft'",
                rusqlite::params![
                    working_days, present_days, leave_days, paid_holidays,
                    basic, hra, transport, other, gross, lop, lop_basic, lop_hra, lop_transport, lop_other,
                    shift_penalty, pf, esi, tds, prof_tax, advance, lw_employee,
                    total_deductions, net, adj_json, payroll_detail, &now, pid,
                ],
            );
            pid
        } else {
            let _ = conn.execute(
                "INSERT INTO payslips (user_id, month, year, working_days, present_days, leave_days, holiday_days,
                 basic_salary, hra, transport_allowance, other_allowances, gross_salary, lop_deduction,
                 lop_basic, lop_hra, lop_transport, lop_other, shift_penalty, pf_deduction, esi_deduction, tds,
                 prof_tax, advance_deduction, lw_employee, total_deductions, net_salary, adjustments, payroll_detail,
                 status, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,'draft',?28,?28)",
                rusqlite::params![
                    user_id, body.month, body.year, working_days, present_days, leave_days,
                    paid_holidays, basic, hra, transport, other, gross, lop, lop_basic, lop_hra, lop_transport, lop_other,
                    shift_penalty, pf, esi, tds, prof_tax, advance, lw_employee,
                    total_deductions, net, adj_json, payroll_detail, &now,
                ],
            );
            conn.last_insert_rowid()
        };

        previews.push(serde_json::json!({
            "id": payslip_id,
            "user_id": user_id,
            "user_name": name,
            "working_days": working_days,
            "present_days": present_days,
            "leave_days": leave_days,
            "absent_days": emp.get("absent_days").and_then(|v| v.as_i64()).unwrap_or(0),
            "shift_penalty": shift_penalty,
            "gross_salary": gross,
            "total_deductions": total_deductions,
            "net_salary": net,
            "skipped": false,
        }));
    }

    HttpResponse::Ok().json(ApiResponse::success(previews))
}

pub async fn generate(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<PayrollGenerateRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut generated = 0i64;
    let mut skipped = 0i64;
    let mut results = Vec::new();

    for payslip_id in &body.payslip_ids {
        let row: Option<(f64, f64, f64, String)> = conn
            .query_row(
                "SELECT gross_salary, net_salary, total_deductions, COALESCE(adjustments,'[]') FROM payslips WHERE id=?1",
                [payslip_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .ok();

        let Some((gross, mut net, mut total_deductions, existing_adj)) = row else {
            skipped += 1;
            results.push(serde_json::json!({"id": payslip_id, "status": "not_found"}));
            continue;
        };

        let mut adj_json = existing_adj.clone();
        if let Some(ref common) = body.common_adjustments {
            if !common.is_empty() {
                let (adj_net, adj_ded, _) =
                    payroll_logic::apply_adjustment_list(gross, net, total_deductions, common);
                net = adj_net;
                total_deductions = adj_ded;
                let mut merged: Vec<serde_json::Value> =
                    serde_json::from_str(&existing_adj).unwrap_or_default();
                merged.extend(common.clone());
                adj_json = serde_json::to_string(&merged).unwrap_or_else(|_| "[]".to_string());
            }
        }

        let updated = conn.execute(
            "UPDATE payslips SET net_salary=?1, total_deductions=?2, adjustments=?3,
             status='generated', generated_at=?4, updated_at=?4 WHERE id=?5 AND status='draft'",
            rusqlite::params![net, total_deductions, adj_json, &now, payslip_id],
        );
        if updated.unwrap_or(0) > 0 {
            generated += 1;
            let user_id: i64 = conn
                .query_row("SELECT user_id FROM payslips WHERE id=?1", [payslip_id], |r| r.get(0))
                .unwrap_or(0);
            let advance: f64 = conn
                .query_row(
                    "SELECT COALESCE(advance_deduction, 0) FROM payslips WHERE id=?1",
                    [payslip_id],
                    |r| r.get(0),
                )
                .unwrap_or(0.0);
            if advance > 0.0 && user_id > 0 {
                let _ = conn.execute(
                    "UPDATE employee_advances SET balance = CASE WHEN balance > ?1 THEN balance - ?1 ELSE 0 END,
                     is_active = CASE WHEN balance > ?1 THEN 1 ELSE 0 END, updated_at=?2
                     WHERE user_id=?3 AND is_active=1",
                    rusqlite::params![advance, &now, user_id],
                );
            }
            results.push(serde_json::json!({"id": payslip_id, "status": "generated", "net_salary": net}));
        } else {
            skipped += 1;
            results.push(serde_json::json!({"id": payslip_id, "status": "skipped"}));
        }
    }

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "message": format!("Generated {} payslips ({} skipped)", generated, skipped),
        "generated": generated,
        "skipped": skipped,
        "results": results,
    })))
}

/// POST /api/admin/payslips/{id}/unlock — revert generated payslip to draft
pub async fn unlock_payslip(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let id = path.into_inner();
    let updated = conn.execute(
        "UPDATE payslips SET status='draft', generated_at=NULL, updated_at=?1 WHERE id=?2 AND status='generated'",
        rusqlite::params![
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            id
        ],
    );
    if updated.unwrap_or(0) == 0 {
        return HttpResponse::NotFound().json(ApiError::new("Generated payslip not found"));
    }
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Payslip unlocked"})))
}
