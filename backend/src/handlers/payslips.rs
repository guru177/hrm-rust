use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::payslip::Payslip;
use crate::models::{ApiError, ApiResponse};

#[derive(Debug, Deserialize)]
pub struct PayslipListQuery {
    pub month: Option<i32>,
    pub year: Option<i32>,
}

const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

/// GET /api/admin/me/payslips — current user's payslips (no special permission)
pub async fn my_payslips_list(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<PayslipListQuery>,
) -> HttpResponse {
    let claims = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    employee_payslips_list_inner(pool, claims.sub, query).await
}

async fn employee_payslips_list_inner(
    pool: web::Data<DbPool>,
    user_id: i64,
    query: web::Query<PayslipListQuery>,
) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM users WHERE id=?1 AND deleted_at IS NULL",
            [user_id],
            |_| Ok(()),
        )
        .is_ok();
    if !exists {
        return HttpResponse::NotFound().json(ApiError::new("Employee not found"));
    }

    let mut sql =
        String::from("SELECT * FROM payslips WHERE user_id=?1 AND status != 'draft'");
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(user_id)];

    if let Some(year) = query.year {
        sql.push_str(" AND year=?");
        params.push(Box::new(year));
    }
    if let Some(month) = query.month {
        sql.push_str(" AND month=?");
        params.push(Box::new(month));
    }
    sql.push_str(" ORDER BY year DESC, month DESC");

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(ApiError::new(&format!("{}", e))),
    };

    let items: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            let p = Payslip::from_row(row)?;
            Ok(serde_json::json!({
                "id": p.id,
                "month": p.month,
                "year": p.year,
                "gross_salary": format!("{:.2}", p.gross_salary),
                "total_deductions": format!("{:.2}", p.total_deductions),
                "net_salary": format!("{:.2}", p.net_salary),
                "status": p.status,
                "generated_at": p.generated_at.or(p.updated_at),
                "created_at": p.created_at,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(items))
}

/// GET /api/admin/salaries/employees/{id}/payslips/list
pub async fn employee_payslips_list(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    query: web::Query<PayslipListQuery>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    employee_payslips_list_inner(pool, path.into_inner(), query).await
}

/// POST /api/admin/payslips/{id}/send-whatsapp
pub async fn send_whatsapp(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let payslip_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let row: Option<(String, Option<String>, i32, i32, f64, f64, String)> = conn
        .query_row(
            "SELECT u.name, u.phone, p.month, p.year, p.net_salary, p.gross_salary, p.status
             FROM payslips p JOIN users u ON u.id = p.user_id WHERE p.id=?1",
            [payslip_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .ok();

    let Some((name, phone, month, year, net, gross, status)) = row else {
        return HttpResponse::NotFound().json(ApiError::new("Payslip not found"));
    };
    if status != "generated" {
        return HttpResponse::BadRequest().json(ApiError::new("Only generated payslips can be sent"));
    }

    let phone = phone.filter(|p| !p.trim().is_empty());
    let Some(phone) = phone else {
        return HttpResponse::BadRequest().json(ApiError::new("Employee has no phone number on file"));
    };

    let msg91_key: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='msg91_auth_key'",
            [],
            |r| r.get(0),
        )
        .ok()
        .flatten()
        .filter(|s: &String| !s.is_empty());

    let Some(auth_key) = msg91_key else {
        return HttpResponse::BadRequest().json(ApiError::new(
            "WhatsApp not configured. Add MSG91 Auth Key in App Settings (key: msg91_auth_key).",
        ));
    };

    let sender: String = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='msg91_whatsapp_sender'",
            [],
            |r| r.get(0),
        )
        .ok()
        .flatten()
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| phone.clone());

    let month_label = MONTH_NAMES
        .get((month as usize).saturating_sub(1))
        .copied()
        .unwrap_or("Month");
    let message = format!(
        "Hello {}, your payslip for {} {} is ready. Gross: {:.2}, Net: {:.2}.",
        name, month_label, year, gross, net
    );

    let phone_digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let payload = serde_json::json!({
        "integrated_number": sender,
        "content_type": "text",
        "payload": {
            "to": phone_digits,
            "type": "text",
            "text": {
                "body": message
            }
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://control.msg91.com/api/v5/whatsapp/whatsapp-outbound-message/")
        .header("authkey", &auth_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({
                "message": "WhatsApp notification sent via MSG91",
                "payslip_id": payslip_id,
            }),
        )),
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            log::error!("MSG91 WhatsApp failed: {}", body);
            HttpResponse::BadGateway().json(ApiError::new(&format!(
                "MSG91 rejected the request: {}",
                if body.is_empty() { "unknown error" } else { &body }
            )))
        }
        Err(e) => HttpResponse::BadGateway().json(ApiError::new(&format!(
            "Failed to reach MSG91: {}",
            e
        ))),
    }
}

fn fmt_inr(n: f64) -> String {
    format!("{:.2}", n)
}

/// GET /api/admin/payslips/{id}/pdf — printable payslip HTML
pub async fn payslip_pdf(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let payslip_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    let row = conn.query_row(
        "SELECT p.id, p.month, p.year, p.gross_salary, p.total_deductions, p.net_salary, p.status,
                p.working_days, p.present_days, p.leave_days, p.holiday_days,
                p.basic_salary, p.hra, p.transport_allowance, p.other_allowances,
                COALESCE(p.lop_deduction, 0), COALESCE(p.shift_penalty, 0),
                COALESCE(p.lop_basic, 0), COALESCE(p.lop_hra, 0), COALESCE(p.lop_transport, 0),
                COALESCE(p.pf_deduction, 0), COALESCE(p.esi_deduction, 0), COALESCE(p.tds, 0),
                COALESCE(p.prof_tax, 0), COALESCE(p.advance_deduction, 0), COALESCE(p.lw_employee, 0),
                COALESCE(p.adjustments, '[]'), u.name, u.employee_id, d.name
         FROM payslips p
         JOIN users u ON u.id = p.user_id
         LEFT JOIN departments d ON d.id = u.department_id
         WHERE p.id=?1",
        [payslip_id],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i32>(1)?,
                r.get::<_, i32>(2)?,
                r.get::<_, f64>(3)?,
                r.get::<_, f64>(4)?,
                r.get::<_, f64>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, i64>(7).unwrap_or(0),
                r.get::<_, i64>(8).unwrap_or(0),
                r.get::<_, i64>(9).unwrap_or(0),
                r.get::<_, i64>(10).unwrap_or(0),
                r.get::<_, f64>(11).unwrap_or(0.0),
                r.get::<_, f64>(12).unwrap_or(0.0),
                r.get::<_, f64>(13).unwrap_or(0.0),
                r.get::<_, f64>(14).unwrap_or(0.0),
                r.get::<_, f64>(15).unwrap_or(0.0),
                r.get::<_, f64>(16).unwrap_or(0.0),
                r.get::<_, f64>(17).unwrap_or(0.0),
                r.get::<_, f64>(18).unwrap_or(0.0),
                r.get::<_, f64>(19).unwrap_or(0.0),
                r.get::<_, f64>(20).unwrap_or(0.0),
                r.get::<_, f64>(21).unwrap_or(0.0),
                r.get::<_, f64>(22).unwrap_or(0.0),
                r.get::<_, f64>(23).unwrap_or(0.0),
                r.get::<_, f64>(24).unwrap_or(0.0),
                r.get::<_, f64>(25).unwrap_or(0.0),
                r.get::<_, String>(26)?,
                r.get::<_, String>(27)?,
                r.get::<_, Option<String>>(28)?,
                r.get::<_, Option<String>>(29)?,
            ))
        },
    );

    let Ok((
        id, month, year, gross, total_ded, net, status,
        working, present, leave, holidays,
        basic, hra, transport, other,
        lop, shift_penalty, lop_basic, lop_hra, lop_transport,
        pf, esi, tds, prof_tax, advance, lw_employee,
        adjustments_json, emp_name, emp_id, dept,
    )) = row else {
        return HttpResponse::NotFound().json(ApiError::new("Payslip not found"));
    };

    let month_label = MONTH_NAMES
        .get((month as usize).saturating_sub(1))
        .copied()
        .unwrap_or("Month");

    let adjustments: Vec<serde_json::Value> =
        serde_json::from_str(&adjustments_json).unwrap_or_default();
    let mut adj_rows = String::new();
    for adj in &adjustments {
        let label = adj.get("label").and_then(|v| v.as_str()).unwrap_or("Adjustment");
        let amount = adj.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let kind = adj.get("type").and_then(|v| v.as_str()).unwrap_or("deduction");
        let sign = if kind == "addition" { "+" } else { "-" };
        adj_rows.push_str(&format!(
            "<tr><td>{label}</td><td class=\"num\">{sign} {amt}</td></tr>",
            label = label,
            sign = sign,
            amt = fmt_inr(amount)
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>Payslip {month_label} {year} — {emp}</title>
<style>
  body {{ font-family: Arial, sans-serif; margin: 32px; color: #111; }}
  h1 {{ margin: 0 0 4px; font-size: 22px; }}
  .meta {{ color: #555; margin-bottom: 24px; }}
  table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
  th, td {{ border: 1px solid #ddd; padding: 8px 10px; text-align: left; }}
  th {{ background: #f5f5f5; }}
  .num {{ text-align: right; font-variant-numeric: tabular-nums; }}
  .summary td {{ font-weight: bold; }}
  @media print {{
    .no-print {{ display: none; }}
    body {{ margin: 12px; }}
  }}
</style>
</head>
<body>
  <div class="no-print" style="margin-bottom:16px;">
    <button onclick="window.print()" style="padding:8px 16px;cursor:pointer;">Print / Save as PDF</button>
  </div>
  <h1>Payslip</h1>
  <div class="meta">
    <div><strong>{emp}</strong>{emp_id}{dept}</div>
    <div>Period: {month_label} {year} &nbsp;|&nbsp; Status: {status} &nbsp;|&nbsp; Payslip #{id}</div>
    <div>Working days: {working} &nbsp; Present: {present} &nbsp; Leave: {leave} &nbsp; Holidays: {holidays}</div>
  </div>

  <h2>Earnings</h2>
  <table>
    <tr><th>Component</th><th class="num">Amount (INR)</th></tr>
    <tr><td>Basic</td><td class="num">{basic}</td></tr>
    <tr><td>HRA</td><td class="num">{hra}</td></tr>
    <tr><td>Transport</td><td class="num">{transport}</td></tr>
    <tr><td>Other Allowances</td><td class="num">{other}</td></tr>
    <tr class="summary"><td>Gross Salary</td><td class="num">{gross}</td></tr>
  </table>

  <h2>Deductions</h2>
  <table>
    <tr><th>Component</th><th class="num">Amount (INR)</th></tr>
    <tr><td>LOP — Basic</td><td class="num">{lop_basic}</td></tr>
    <tr><td>LOP — HRA</td><td class="num">{lop_hra}</td></tr>
    <tr><td>LOP — Conveyance</td><td class="num">{lop_transport}</td></tr>
    <tr><td>LOP Total</td><td class="num">{lop}</td></tr>
    <tr><td>Late/Early Penalty</td><td class="num">{shift_penalty}</td></tr>
    <tr><td>EPF (Employee)</td><td class="num">{pf}</td></tr>
    <tr><td>ESI (Employee)</td><td class="num">{esi}</td></tr>
    <tr><td>Professional Tax</td><td class="num">{prof_tax}</td></tr>
    <tr><td>Labour Welfare</td><td class="num">{lw_employee}</td></tr>
    <tr><td>Advance Recovery</td><td class="num">{advance}</td></tr>
    <tr><td>TDS</td><td class="num">{tds}</td></tr>
    {adj_rows}
    <tr class="summary"><td>Total Deductions</td><td class="num">{total_ded}</td></tr>
  </table>

  <table>
    <tr class="summary"><td>Net Salary</td><td class="num">{net}</td></tr>
  </table>
</body>
</html>"#,
        emp = emp_name,
        emp_id = emp_id
            .as_ref()
            .map(|e| format!(" &nbsp;|&nbsp; ID: {}", e))
            .unwrap_or_default(),
        dept = dept
            .as_ref()
            .map(|d| format!(" &nbsp;|&nbsp; {}", d))
            .unwrap_or_default(),
        month_label = month_label,
        year = year,
        status = status,
        id = id,
        working = working,
        present = present,
        leave = leave,
        holidays = holidays,
        basic = fmt_inr(basic),
        hra = fmt_inr(hra),
        transport = fmt_inr(transport),
        other = fmt_inr(other),
        gross = fmt_inr(gross),
        lop = fmt_inr(lop),
        lop_basic = fmt_inr(lop_basic),
        lop_hra = fmt_inr(lop_hra),
        lop_transport = fmt_inr(lop_transport),
        shift_penalty = fmt_inr(shift_penalty),
        pf = fmt_inr(pf),
        esi = fmt_inr(esi),
        prof_tax = fmt_inr(prof_tax),
        lw_employee = fmt_inr(lw_employee),
        advance = fmt_inr(advance),
        tds = fmt_inr(tds),
        adj_rows = adj_rows,
        total_ded = fmt_inr(total_ded),
        net = fmt_inr(net),
    );

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header(("Content-Disposition", format!("inline; filename=\"payslip-{}-{:02}-{}.html\"", year, month, id)))
        .body(html)
}
