use actix_web::{web, HttpRequest, HttpResponse};
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::{ApiError, ApiResponse};
use crate::models::salary::SalaryComponent;
use serde::Deserialize;

fn row_component_amount(row: &rusqlite::Row) -> Option<f64> {
    row.get::<_, Option<f64>>("default_value")
        .ok()
        .flatten()
        .or_else(|| row.get::<_, Option<f64>>("amount").ok().flatten())
        .or_else(|| row.get::<_, Option<f64>>("max_amount_per_month").ok().flatten())
}

fn normalize_calculation_type(raw: Option<&str>) -> String {
    match raw {
        Some("percentage_of_basic") => "percentage_of_basic".to_string(),
        Some("percentage_of_ctc") => "percentage_of_ctc".to_string(),
        Some("percentage_of_gross") => "percentage_of_gross".to_string(),
        Some("reimbursement") => "flat_amount".to_string(),
        Some("flat_amount") => "flat_amount".to_string(),
        _ => "flat_amount".to_string(),
    }
}

fn calc_type_label(calc: &str) -> &'static str {
    match calc {
        "percentage_of_basic" => "Percentage of Basic",
        "percentage_of_ctc" => "Percentage of CTC",
        "percentage_of_gross" => "Percentage of Gross",
        _ => "Flat Amount",
    }
}

fn statutory_preview_json(p: &crate::salary_split::CtcStatutoryPreview) -> serde_json::Value {
    serde_json::to_value(p).unwrap_or_else(|_| serde_json::json!({}))
}

fn row_is_reimbursement(row: &rusqlite::Row, component_type: &str) -> bool {
    if component_type == "reimbursement" {
        return true;
    }
    row.get::<_, Option<String>>("reimbursement_type")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .is_some()
        || row.get::<_, Option<String>>("calculation_type")
            .ok()
            .flatten()
            .as_deref()
            == Some("reimbursement")
}

/// List salary components filtered by type (earning/deduction/reimbursement)
pub async fn components_list(pool: web::Data<DbPool>, req: HttpRequest, query: web::Query<ComponentQuery>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let sql = if let Some(ref ctype) = query.r#type {
        if ctype != "earning" && ctype != "deduction" && ctype != "reimbursement" {
            return HttpResponse::BadRequest().json(ApiError::new("Invalid component_type"));
        }
        if ctype == "reimbursement" {
            "SELECT * FROM salary_components
             WHERE COALESCE(component_type, type)='reimbursement'
                OR reimbursement_type IS NOT NULL
                OR calculation_type='reimbursement'
             ORDER BY name"
        } else {
            "SELECT * FROM salary_components
             WHERE COALESCE(component_type, type)=?1
               AND COALESCE(component_type, type) != 'reimbursement'
               AND reimbursement_type IS NULL
               AND (calculation_type IS NULL OR calculation_type != 'reimbursement')
             ORDER BY name"
        }
    } else {
        "SELECT * FROM salary_components ORDER BY name"
    };

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
    };
    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<serde_json::Value> {
        let c = SalaryComponent::from_row(row)?;
        let comp_type = if row_is_reimbursement(row, c.component_type.as_str()) {
            "reimbursement".to_string()
        } else {
            c.component_type.clone()
        };
        let calc_type = normalize_calculation_type(c.calculation_type.as_deref());
        let amount = row_component_amount(row);
        Ok(serde_json::json!({
            "id": c.id,
            "name": c.name,
            "type": comp_type,
            "description": row.get::<_, Option<String>>("description").ok().flatten(),
            "is_active": row.get::<_, Option<i64>>("is_active").ok().flatten().unwrap_or(1) != 0,
            "earning_type": row.get::<_, Option<String>>("earning_type").ok().flatten(),
            "name_in_payslip": row.get::<_, Option<String>>("name_in_payslip").ok().flatten().unwrap_or_else(|| c.name.clone()),
            "calculation_type": calc_type,
            "amount": amount.map(|v| v.to_string()),
            "deduction_type": row.get::<_, Option<String>>("deduction_type").ok().flatten(),
            "deduction_frequency": row.get::<_, Option<String>>("deduction_frequency").ok().flatten().unwrap_or_else(|| "recurring".into()),
            "is_pre_tax": c.is_taxable,
            "reimbursement_type": row.get::<_, Option<String>>("reimbursement_type").ok().flatten(),
            "max_amount_per_month": amount.map(|v| v.to_string()),
            "component_type": c.component_type,
            "default_value": c.default_value,
            "is_taxable": c.is_taxable,
        }))
    };
    let items: Vec<serde_json::Value> = if let Some(ref ctype) = query.r#type {
        if ctype == "reimbursement" {
            stmt.query_map([], map_row)
        } else {
            stmt.query_map([ctype], map_row)
        }
    } else {
        stmt.query_map([], map_row)
    }
    .unwrap()
    .filter_map(|r| r.ok())
    .collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

#[derive(Debug, Deserialize)]
pub struct ComponentQuery {
    pub r#type: Option<String>,
}

fn deserialize_opt_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct OptF64;
    impl<'de> Visitor<'de> for OptF64 {
        type Value = Option<f64>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a number or numeric string")
        }
        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v))
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            v.parse::<f64>()
                .map(Some)
                .map_err(|_| de::Error::custom(format!("invalid number: {v}")))
        }
    }
    deserializer.deserialize_any(OptF64)
}

#[derive(Debug, Deserialize)]
pub struct CreateComponentRequest {
    pub name: String,
    #[serde(alias = "type")]
    pub component_type: String,
    pub description: Option<String>,
    pub calculation_type: Option<String>,
    #[serde(alias = "amount", default, deserialize_with = "deserialize_opt_f64")]
    pub default_value: Option<f64>,
    #[serde(alias = "is_pre_tax")]
    pub is_taxable: Option<bool>,
    pub is_active: Option<bool>,
    pub earning_type: Option<String>,
    pub name_in_payslip: Option<String>,
    pub deduction_type: Option<String>,
    pub deduction_frequency: Option<String>,
    pub reimbursement_type: Option<String>,
    #[serde(alias = "max_amount_per_month", default, deserialize_with = "deserialize_opt_f64")]
    pub max_amount: Option<f64>,
}

struct ResolvedComponent {
    comp_type: String,
    calc_type: Option<String>,
    amount: Option<f64>,
    is_taxable: bool,
    is_active: bool,
    earning_type: Option<String>,
    name_in_payslip: Option<String>,
    deduction_type: Option<String>,
    deduction_frequency: Option<String>,
    reimbursement_type: Option<String>,
    max_amount: Option<f64>,
}

fn resolve_component_fields(body: &CreateComponentRequest) -> ResolvedComponent {
    let ui_type = body.component_type.as_str();
    let comp_type = if ui_type == "reimbursement" {
        "reimbursement".to_string()
    } else {
        body.component_type.clone()
    };
    let calc_type = normalize_calculation_type(body.calculation_type.as_deref());
    let amount = body.default_value.or(body.max_amount);
    let max_amount = if ui_type == "reimbursement" && calc_type == "flat_amount" {
        amount
    } else {
        body.max_amount
    };
    ResolvedComponent {
        comp_type: comp_type.clone(),
        calc_type: Some(calc_type),
        amount,
        is_taxable: body.is_taxable.unwrap_or(ui_type != "deduction"),
        is_active: body.is_active.unwrap_or(true),
        earning_type: if ui_type == "earning" {
            body.earning_type.clone()
        } else {
            None
        },
        name_in_payslip: body
            .name_in_payslip
            .clone()
            .or_else(|| Some(body.name.clone())),
        deduction_type: if ui_type == "deduction" {
            body.deduction_type.clone()
        } else {
            None
        },
        deduction_frequency: body.deduction_frequency.clone(),
        reimbursement_type: if ui_type == "reimbursement" {
            body.reimbursement_type.clone()
        } else {
            None
        },
        max_amount,
    }
}

fn persist_component(
    conn: &rusqlite::Connection,
    id: Option<i64>,
    body: &CreateComponentRequest,
    now: &str,
) -> Result<(), rusqlite::Error> {
    let slug = body.name.to_lowercase().replace(' ', "_");
    let r = resolve_component_fields(body);
    let is_active = if r.is_active { 1 } else { 0 };
    let is_pre_tax = if r.is_taxable { 1 } else { 0 };
    let is_taxable = is_pre_tax;

    if let Some(cid) = id {
        conn.execute(
            "UPDATE salary_components SET
                name=?1, type=?2, description=?3, is_active=?4, updated_at=?5,
                earning_type=?6, name_in_payslip=?7, calculation_type=?8, amount=?9,
                deduction_type=?10, deduction_frequency=?11, is_pre_tax=?12,
                reimbursement_type=?13, max_amount_per_month=?14,
                slug=?15, component_type=?16, default_value=?17, is_taxable=?18
             WHERE id=?19",
            rusqlite::params![
                body.name,
                r.comp_type,
                body.description,
                is_active,
                now,
                r.earning_type,
                r.name_in_payslip,
                r.calc_type,
                r.amount,
                r.deduction_type,
                r.deduction_frequency,
                is_pre_tax,
                r.reimbursement_type,
                r.max_amount,
                slug,
                r.comp_type,
                r.amount,
                is_taxable,
                cid,
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO salary_components (
                name, type, description, is_active, created_at, updated_at,
                earning_type, name_in_payslip, calculation_type, amount,
                deduction_type, deduction_frequency, is_pre_tax,
                reimbursement_type, max_amount_per_month,
                slug, component_type, default_value, is_taxable
             ) VALUES (?1,?2,?3,?4,?5,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            rusqlite::params![
                body.name,
                r.comp_type,
                body.description,
                is_active,
                now,
                r.earning_type,
                r.name_in_payslip,
                r.calc_type,
                r.amount,
                r.deduction_type,
                r.deduction_frequency,
                is_pre_tax,
                r.reimbursement_type,
                r.max_amount,
                slug,
                r.comp_type,
                r.amount,
                is_taxable,
            ],
        )?;
    }
    Ok(())
}

pub async fn components_store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<CreateComponentRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match persist_component(&conn, None, &body, &now) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(serde_json::json!({"id": conn.last_insert_rowid()}))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}

pub async fn components_update(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>, body: web::Json<CreateComponentRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = path.into_inner();
    match persist_component(&conn, Some(id), &body, &now) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Updated"}))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}

pub async fn components_destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let _ = conn.execute("DELETE FROM salary_components WHERE id=?1", [path.into_inner()]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Deleted"})))
}

/// List employees with salary structures — returns paginated format
pub async fn employees_list(pool: web::Data<DbPool>, req: HttpRequest, query: web::Query<EmployeeListQuery>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let per_page = query.per_page.unwrap_or(15).min(100) as i64;
    let page = query.page.unwrap_or(1).max(1) as i64;
    let offset = (page - 1) * per_page;

    let mut conditions = vec!["u.deleted_at IS NULL".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    let status = query.status.as_deref().unwrap_or("active");
    if status != "all" {
        conditions.push("u.status = ?".to_string());
        params.push(Box::new(status.to_string()));
    }
    if let Some(ref search) = query.search {
        if !search.is_empty() {
            conditions.push("(u.name LIKE ? OR u.email LIKE ? OR u.employee_id LIKE ?)".to_string());
            let like = format!("%{}%", search);
            params.push(Box::new(like.clone()));
            params.push(Box::new(like.clone()));
            params.push(Box::new(like));
        }
    }
    if let Some(ref dept) = query.department_id {
        if dept != "all" {
            if let Ok(dept_id) = dept.parse::<i64>() {
                conditions.push("u.department_id = ?".to_string());
                params.push(Box::new(dept_id));
            }
        }
    }
    if let Some(ref desig) = query.designation_id {
        if desig != "all" {
            if let Ok(desig_id) = desig.parse::<i64>() {
                conditions.push("u.designation_id = ?".to_string());
                params.push(Box::new(desig_id));
            }
        }
    }
    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM users u WHERE {}", where_clause);
    let total: i64 = conn
        .query_row(
            &count_sql,
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |r| r.get(0),
        )
        .unwrap_or(0);
    let last_page = ((total as f64) / (per_page as f64)).ceil().max(1.0) as i64;
    let from = if total > 0 { offset + 1 } else { 0 };
    let to = (offset + per_page).min(total);

    let sql = format!(
        "SELECT u.id, u.name, u.email, u.employee_id, u.status, u.avatar, u.photo,
                d.id AS dept_id, d.name AS dept_name,
                des.id AS desig_id, des.name AS desig_name,
                (SELECT MAX(COALESCE(generated_at, updated_at)) FROM payslips
                 WHERE user_id = u.id AND status = 'generated') AS last_payslip_date
         FROM users u
         LEFT JOIN departments d ON d.id = u.department_id
         LEFT JOIN designations des ON des.id = u.designation_id
         WHERE {} ORDER BY u.name LIMIT ? OFFSET ?",
        where_clause
    );
    params.push(Box::new(per_page));
    params.push(Box::new(offset));

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows: Vec<(i64, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>, Option<String>, Option<i64>, Option<String>, Option<String>)> = stmt
        .query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7).ok(),
                row.get(8).ok(),
                row.get(9).ok(),
                row.get(10).ok(),
                row.get(11).ok(),
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(user_id, name, email, employee_id, status, avatar, photo, dept_id, dept_name, desig_id, desig_name, last_payslip)| {
            let salary = crate::salary_logic::load_user_salary(&conn, user_id, &today);
            let salary_val = salary
                .map(|s| serde_json::json!(format!("{:.2}", s.gross)))
                .unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "id": user_id,
                "name": name,
                "email": email,
                "employee_id": employee_id,
                "status": status,
                "salary": salary_val,
                "avatar": avatar,
                "photo": photo,
                "department": dept_id.map(|id| serde_json::json!({
                    "id": id,
                    "name": dept_name.unwrap_or_default(),
                })),
                "designation": desig_id.map(|id| serde_json::json!({
                    "id": id,
                    "name": desig_name.unwrap_or_default(),
                })),
                "last_payslip_date": last_payslip,
            })
        })
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "data": items,
        "total": total,
        "last_page": last_page,
        "current_page": page,
        "per_page": per_page,
        "from": from,
        "to": to,
    })))
}

#[derive(Debug, Deserialize)]
pub struct EmployeeListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub department_id: Option<String>,
    pub designation_id: Option<String>,
}

pub async fn employees_filter_options(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c)=>c, Err(e)=>return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c)=>c, Err(_)=>return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let mut dept_stmt = conn.prepare("SELECT id, name FROM departments ORDER BY name").unwrap();
    let departments: Vec<serde_json::Value> = dept_stmt.query_map([], |row| {
        Ok(serde_json::json!({"id": row.get::<_, i64>(0)?, "name": row.get::<_, String>(1)?}))
    }).unwrap().filter_map(|r| r.ok()).collect();

    let mut desig_stmt = conn.prepare("SELECT id, name FROM designations ORDER BY name").unwrap();
    let designations: Vec<serde_json::Value> = desig_stmt.query_map([], |row| {
        Ok(serde_json::json!({"id": row.get::<_, i64>(0)?, "name": row.get::<_, String>(1)?}))
    }).unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "departments": departments,
        "designations": designations,
        "statuses": ["active", "inactive", "on_leave"]
    })))
}

// ── Per-user salary structure (component assignments) ──

#[derive(Debug, Deserialize)]
pub struct StoreUserSalaryStructureRequest {
    pub effective_from: String,
    pub items: Vec<UserSalaryStructureItem>,
}

#[derive(Debug, Deserialize)]
pub struct UserSalaryStructureItem {
    pub salary_component_id: i64,
    pub amount: f64,
}

fn user_exists(conn: &rusqlite::Connection, user_id: i64) -> bool {
    conn.query_row(
        "SELECT 1 FROM users WHERE id=?1 AND deleted_at IS NULL",
        [user_id],
        |_| Ok(()),
    )
    .is_ok()
}

fn build_user_salary_structure(conn: &rusqlite::Connection, user_id: i64) -> serde_json::Value {
    let as_of = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let ctc_profile = crate::salary_split::load_employee_profile(conn, user_id, &as_of);
    let monthly_ctc = ctc_profile.as_ref().map(|p| crate::salary_split::round2(p.yearly_ctc / 12.0));
    let gross_monthly = ctc_profile.as_ref().map(|p| {
        crate::salary_split::preview_for_profile(conn, p).gross
    });

    let effective_from: String = ctc_profile
        .as_ref()
        .map(|p| p.effective_from.clone())
        .or_else(|| {
            conn.query_row(
                "SELECT effective_from FROM salary_structure_items WHERE user_id=?1 ORDER BY effective_from DESC LIMIT 1",
                [user_id],
                |r| r.get(0),
            )
            .ok()
        })
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());

    let mut stmt = conn
        .prepare(
            "SELECT sc.id, sc.name, COALESCE(sc.component_type, sc.type) AS comp_type,
                    sc.calculation_type,
                    COALESCE(sc.default_value, sc.amount) AS default_value,
                    sc.is_taxable, ssi.amount AS assigned_amount,
                    sc.deduction_frequency, LOWER(COALESCE(sc.slug, '')) AS slug
             FROM salary_components sc
             LEFT JOIN salary_structure_items ssi
               ON ssi.salary_component_id = sc.id AND ssi.user_id = ?1 AND ssi.effective_from = ?2
             ORDER BY CASE COALESCE(sc.component_type, sc.type)
                 WHEN 'earning' THEN 0 WHEN 'deduction' THEN 1 ELSE 2 END, sc.name",
        )
        .unwrap();

    let mut components: Vec<serde_json::Value> = Vec::new();
    let mut gross_salary = 0.0f64;
    let mut total_deductions = 0.0f64;
    let mut basic_amount = 0.0f64;

    let rows: Vec<_> = stmt
        .query_map(rusqlite::params![user_id, &effective_from], |row| {
            let comp_type: String = row.get(2)?;
            let stored: Option<f64> = row.get(6)?;
            let default_amt: Option<f64> = row.get(4)?;
            let raw_calc: Option<String> = row.get(3)?;
            let calc_type = normalize_calculation_type(raw_calc.as_deref());
            let slug: String = row.get(8)?;
            let pct = default_amt.unwrap_or(0.0);

            Ok((
                comp_type,
                stored,
                calc_type,
                pct,
                slug,
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(5).unwrap_or(0) != 0,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // First pass: resolve basic for percentage_of_basic dependencies
    for row in &rows {
        let (comp_type, stored, calc_type, pct, slug, _, name, ..) = row;
        if comp_type != "earning" {
            continue;
        }
        let amt = stored.unwrap_or_else(|| {
            let mctc = monthly_ctc.unwrap_or(0.0);
            let g = gross_monthly.unwrap_or(mctc);
            if calc_type == "percentage_of_gross" {
                return crate::salary_split::amount_from_calc(calc_type, *pct, mctc, g, 0.0);
            }
            if calc_type == "percentage_of_ctc" {
                if monthly_ctc.is_some() {
                    return crate::salary_split::amount_from_calc(calc_type, *pct, mctc, g, 0.0);
                }
            }
            if monthly_ctc.is_some() {
                crate::salary_split::amount_from_calc(calc_type, *pct, mctc, g, 0.0)
            } else {
                *pct
            }
        });
        if slug.contains("basic") || name.to_lowercase().contains("basic") {
            basic_amount = amt;
            break;
        }
    }
    if basic_amount <= 0.0 {
        if let Some(mctc) = monthly_ctc {
            basic_amount = crate::salary_split::round2(mctc * 0.5);
        }
    }

    for row in rows {
        let (comp_type, stored, calc_type, pct, _slug, comp_id, name, is_taxable, ded_freq) = row;
        let resolved = stored.unwrap_or_else(|| {
            let mctc = monthly_ctc.unwrap_or(0.0);
            let g = gross_monthly.unwrap_or(mctc);
            if calc_type == "percentage_of_gross" {
                return crate::salary_split::amount_from_calc(&calc_type, pct, mctc, g, basic_amount);
            }
            if calc_type == "percentage_of_ctc" {
                if monthly_ctc.is_some() {
                    return crate::salary_split::amount_from_calc(&calc_type, pct, mctc, g, basic_amount);
                }
            }
            if monthly_ctc.is_some() {
                crate::salary_split::amount_from_calc(&calc_type, pct, mctc, g, basic_amount)
            } else if calc_type.starts_with("percentage_") {
                0.0
            } else {
                pct
            }
        });
        let is_assigned = stored.is_some() || (monthly_ctc.is_some() && calc_type.starts_with("percentage_"));

        let component_default_amount = if calc_type.starts_with("percentage_") {
            Some(if pct.fract() == 0.0 {
                format!("{}", pct as i64)
            } else {
                format!("{}", pct)
            })
        } else {
            Some(if pct.fract() == 0.0 {
                format!("{}", pct as i64)
            } else {
                format!("{}", pct)
            })
        };

        if is_assigned {
            if comp_type == "earning" {
                gross_salary += resolved;
            } else if comp_type == "deduction" {
                total_deductions += resolved;
            }
        }

        components.push(serde_json::json!({
            "salary_component_id": comp_id,
            "name": name,
            "type": comp_type,
            "calculation_type": calc_type,
            "calculation_type_label": calc_type_label(&calc_type),
            "component_default_amount": component_default_amount,
            "is_taxable": is_taxable,
            "is_pre_tax": is_taxable,
            "deduction_frequency": ded_freq,
            "assigned_amount": if is_assigned { Some(resolved) } else { None },
            "is_assigned": is_assigned,
        }));
    }

    serde_json::json!({
        "components": components,
        "effective_from": effective_from,
        "gross_salary": crate::salary_split::round2(gross_salary),
        "total_deductions": crate::salary_split::round2(total_deductions),
        "net_salary": crate::salary_split::round2((gross_salary - total_deductions).max(0.0)),
        "ctc_locked": ctc_profile.is_some(),
        "read_only_reason": if ctc_profile.is_some() {
            Some("Payroll is driven by CTC Split. Edit CTC or remove the CTC profile to use manual components.")
        } else {
            None
        },
        "ctc_profile": ctc_profile.map(|p| {
            let comp = crate::salary_split::load_component_split_config(conn);
            serde_json::json!({
                "yearly_ctc": p.yearly_ctc,
                "monthly_ctc": crate::salary_split::round2(p.yearly_ctc / 12.0),
                "split_source": "salary_components",
                "earnings": comp.earnings.iter().map(|e| serde_json::json!({
                    "id": e.id,
                    "name": e.name,
                    "calculation_type": e.calc_type,
                    "pct": e.pct,
                    "flat": e.flat,
                })).collect::<Vec<_>>(),
            })
        }),
    })
}

/// GET /api/admin/users/{id}/salary-structure
pub async fn user_salary_structure_show(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let user_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    if !user_exists(&conn, user_id) {
        return HttpResponse::NotFound().json(ApiError::new("User not found"));
    }

    HttpResponse::Ok().json(ApiResponse::success(build_user_salary_structure(&conn, user_id)))
}

/// POST /api/admin/users/{id}/salary-structure
pub async fn user_salary_structure_store(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<StoreUserSalaryStructureRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let user_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };

    if !user_exists(&conn, user_id) {
        return HttpResponse::NotFound().json(ApiError::new("User not found"));
    }

    if crate::salary_split::load_employee_profile(&conn, user_id, &body.effective_from).is_some() {
        return HttpResponse::BadRequest().json(ApiError::new(
            "Cannot edit manual structure while CTC is configured. Update CTC Split or remove the CTC profile first.",
        ));
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return HttpResponse::InternalServerError().json(ApiError::new(&format!("{}", e))),
    };
    if let Err(e) = tx.execute(
        "DELETE FROM salary_structure_items WHERE user_id=?1 AND effective_from=?2",
        rusqlite::params![user_id, &body.effective_from],
    ) {
        return HttpResponse::InternalServerError().json(ApiError::new(&format!("{}", e)));
    }

    let now_ts = now.as_str();
    for item in &body.items {
        if let Err(e) = tx.execute(
            "INSERT INTO salary_structure_items (user_id, salary_component_id, amount, effective_from, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            rusqlite::params![
                user_id,
                item.salary_component_id,
                item.amount,
                &body.effective_from,
                now_ts,
            ],
        ) {
            return HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e)));
        }
    }
    if let Err(e) = tx.commit() {
        return HttpResponse::InternalServerError().json(ApiError::new(&format!("{}", e)));
    }

    HttpResponse::Ok().json(ApiResponse::success(build_user_salary_structure(
        &conn, user_id,
    )))
}

// ── Phase 1: CTC templates & employee profiles ──

pub async fn templates_list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let mut stmt = match conn.prepare(
        "SELECT id, name, basic_pct, hra_pct, conv_pct, special_pct, is_default FROM salary_templates ORDER BY name",
    ) {
        Ok(s) => s,
        Err(_) => return HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new())),
    };
    let items: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "basic_pct": row.get::<_, f64>(2)?,
                "hra_pct": row.get::<_, f64>(3)?,
                "conv_pct": row.get::<_, f64>(4)?,
                "special_pct": row.get::<_, f64>(5)?,
                "is_default": row.get::<_, i64>(6).unwrap_or(0) != 0,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

#[derive(Debug, Deserialize)]
pub struct StoreCtcProfileRequest {
    pub yearly_ctc: f64,
    pub effective_from: String,
    pub pf_applicable: Option<bool>,
    pub esi_applicable: Option<bool>,
}

/// GET /api/admin/users/{id}/ctc-profile
pub async fn user_ctc_profile_show(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let user_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    if !user_exists(&conn, user_id) {
        return HttpResponse::NotFound().json(ApiError::new("User not found"));
    }
    let as_of = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let profile = crate::salary_split::load_employee_profile(&conn, user_id, &as_of);
    let comp = crate::salary_split::load_component_split_config(&conn);

    let split_preview = profile.as_ref().map(|p| {
        crate::salary_split::preview_for_profile(&conn, p)
    });

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "profile": profile.map(|p| serde_json::json!({
            "yearly_ctc": p.yearly_ctc,
            "effective_from": p.effective_from,
            "pf_applicable": p.pf_applicable,
            "esi_applicable": p.esi_applicable,
        })),
        "split_preview": split_preview.as_ref().map(statutory_preview_json),
        "salary_components": {
            "has_pf": comp.has_pf,
            "has_esi": comp.has_esi,
            "earnings": comp.earnings.iter().map(|e| serde_json::json!({
                "id": e.id,
                "name": e.name,
                "calculation_type": e.calc_type,
                "pct": e.pct,
                "flat": e.flat,
            })).collect::<Vec<_>>(),
            "deductions": comp.deductions.iter().map(|d| serde_json::json!({
                "id": d.id,
                "name": d.name,
                "calculation_type": d.calc_type,
                "amount": d.amount,
            })).collect::<Vec<_>>(),
        },
    })))
}

#[derive(Debug, Deserialize)]
pub struct CtcPreviewRequest {
    pub yearly_ctc: f64,
    pub pf_applicable: Option<bool>,
    pub esi_applicable: Option<bool>,
}

/// POST /api/admin/salaries/ctc-preview — live statutory CTC breakdown
pub async fn ctc_preview(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<CtcPreviewRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    if body.yearly_ctc <= 0.0 {
        return HttpResponse::BadRequest().json(ApiError::new("Yearly CTC must be positive"));
    }
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let comp = crate::salary_split::load_component_split_config(&conn);
    let cfg = crate::statutory_logic::load_statutory_config(&conn);
    let pf = body.pf_applicable.unwrap_or(true) && comp.has_pf;
    let esi = body.esi_applicable.unwrap_or(true) && comp.has_esi;
    let preview = crate::salary_split::split_with_statutory_from_components(
        &comp,
        body.yearly_ctc,
        &cfg,
        pf,
        esi,
    );
    HttpResponse::Ok().json(ApiResponse::success(statutory_preview_json(&preview)))
}

/// POST /api/admin/users/{id}/ctc-profile
pub async fn user_ctc_profile_store(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<StoreCtcProfileRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let user_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    if !user_exists(&conn, user_id) {
        return HttpResponse::NotFound().json(ApiError::new("User not found"));
    }
    if body.yearly_ctc <= 0.0 {
        return HttpResponse::BadRequest().json(ApiError::new("Yearly CTC must be positive"));
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let comp = crate::salary_split::load_component_split_config(&conn);
    let pf = if comp.has_pf {
        body.pf_applicable.unwrap_or(true)
    } else {
        false
    };
    let esi = if comp.has_esi {
        body.esi_applicable.unwrap_or(true)
    } else {
        false
    };
    let pf_i = if pf { 1 } else { 0 };
    let esi_i = if esi { 1 } else { 0 };

    let _ = conn.execute(
        "DELETE FROM employee_salary_profiles WHERE user_id=?1",
        [user_id],
    );
    if let Err(e) = conn.execute(
        "INSERT INTO employee_salary_profiles (user_id, yearly_ctc, template_id, pf_applicable, esi_applicable, effective_from, created_at, updated_at)
         VALUES (?1,?2,NULL,?3,?4,?5,?6,?6)",
        rusqlite::params![user_id, body.yearly_ctc, pf_i, esi_i, &body.effective_from, &now],
    ) {
        return HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e)));
    }

    let profile = crate::salary_split::EmployeeSalaryProfile {
        user_id,
        yearly_ctc: body.yearly_ctc,
        template_id: None,
        pf_applicable: pf,
        esi_applicable: esi,
        effective_from: body.effective_from.clone(),
    };
    let preview = crate::salary_split::preview_for_profile(&conn, &profile);
    if let Err(e) = crate::salary_split::sync_structure_from_preview(&conn, user_id, &body.effective_from, &preview) {
        log::warn!("CTC structure sync: {}", e);
    }

    user_ctc_profile_show(pool, req, web::Path::from(user_id)).await
}

/// DELETE /api/admin/users/{id}/ctc-profile — remove CTC so manual structure applies
pub async fn user_ctc_profile_destroy(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let user_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    if !user_exists(&conn, user_id) {
        return HttpResponse::NotFound().json(ApiError::new("User not found"));
    }
    let deleted = conn.execute(
        "DELETE FROM employee_salary_profiles WHERE user_id=?1",
        [user_id],
    );
    match deleted {
        Ok(0) => HttpResponse::NotFound().json(ApiError::new("No CTC profile found for this employee")),
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "CTC profile removed. Manual salary structure will be used for payroll.",
        }))),
        Err(e) => HttpResponse::InternalServerError().json(ApiError::new(&format!("{}", e))),
    }
}

// ── Phase 4: Employee advances ──

#[derive(Debug, Deserialize)]
pub struct StoreAdvanceRequest {
    pub amount: f64,
    pub monthly_emi: f64,
    pub description: Option<String>,
}

pub async fn advances_list(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let user_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let mut stmt = match conn.prepare(
        "SELECT id, amount, balance, monthly_emi, description, is_active, created_at
         FROM employee_advances WHERE user_id=?1 ORDER BY id DESC",
    ) {
        Ok(s) => s,
        Err(_) => return HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new())),
    };
    let items: Vec<serde_json::Value> = stmt
        .query_map([user_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "amount": row.get::<_, f64>(1)?,
                "balance": row.get::<_, f64>(2)?,
                "monthly_emi": row.get::<_, f64>(3)?,
                "description": row.get::<_, Option<String>>(4)?,
                "is_active": row.get::<_, i64>(5).unwrap_or(0) != 0,
                "created_at": row.get::<_, Option<String>>(6)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    HttpResponse::Ok().json(ApiResponse::success(items))
}

pub async fn advances_store(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<StoreAdvanceRequest>,
) -> HttpResponse {
    let _c = match get_claims_from_request(&req) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())),
    };
    let user_id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")),
    };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match conn.execute(
        "INSERT INTO employee_advances (user_id, amount, balance, monthly_emi, description, is_active, created_at, updated_at)
         VALUES (?1,?2,?2,?3,?4,1,?5,?5)",
        rusqlite::params![user_id, body.amount, body.monthly_emi, body.description, &now],
    ) {
        Ok(_) => advances_list(pool, req, web::Path::from(user_id)).await,
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("{}", e))),
    }
}
