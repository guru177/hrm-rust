//! Shared salary loading for payroll, analytics, and user salary APIs.

#[derive(Debug, Clone, Default)]
pub struct PayrollSalaryBreakdown {
    pub basic: f64,
    pub hra: f64,
    pub transport: f64,
    pub other_earnings: f64,
    pub pf: f64,
    pub esi: f64,
    pub tds: f64,
    pub other_deductions: f64,
    pub gross: f64,
    pub reimbursement: f64,
    pub fixed_deductions: f64,
    pub components: Vec<serde_json::Value>,
    pub effective_from: Option<String>,
    pub source: String,
}

pub fn bucket_component(slug: &str, name: &str) -> &'static str {
    let s = slug.to_lowercase();
    let n = name.to_lowercase();
    if s.contains("basic") || n.contains("basic") {
        "basic"
    } else if s.contains("hra") || n.contains("house rent") || n.contains("hra") {
        "hra"
    } else if s.contains("transport") || s.contains("travel") || s.contains("conveyance")
        || n.contains("transport") || n.contains("travel") || n.contains("conveyance")
    {
        "transport"
    } else if s.contains("pf") || n.contains("provident") {
        "pf"
    } else if s.contains("esi") {
        "esi"
    } else if s.contains("tds") || n.contains("tax") {
        "tds"
    } else {
        "other"
    }
}

impl PayrollSalaryBreakdown {
    /// Gross earnings used for LOP daily rate (excludes reimbursements).
    pub fn lop_gross(&self) -> f64 {
        (self.gross - self.reimbursement).max(0.0)
    }
}

fn is_reimbursement(calc_type: Option<&str>, slug: &str, name: &str) -> bool {
    calc_type == Some("reimbursement")
        || slug.to_lowercase().contains("reimburse")
        || name.to_lowercase().contains("reimburse")
}

fn is_reimbursement_type(comp_type: &str, calc_type: Option<&str>, slug: &str, name: &str) -> bool {
    comp_type == "reimbursement" || is_reimbursement(calc_type, slug, name)
}

/// Load salary from `salary_structure_items` + `salary_components` (primary path).
pub fn load_from_structure_items(
    conn: &rusqlite::Connection,
    user_id: i64,
    as_of: &str,
) -> Option<PayrollSalaryBreakdown> {
    let effective_from: String = conn
        .query_row(
            "SELECT effective_from FROM salary_structure_items
             WHERE user_id=?1 AND effective_from <= ?2
             ORDER BY effective_from DESC LIMIT 1",
            rusqlite::params![user_id, as_of],
            |r| r.get(0),
        )
        .ok()?;

    let mut stmt = conn
        .prepare(
            "SELECT sc.id, sc.name, sc.slug, COALESCE(sc.component_type, sc.type) AS comp_type, sc.calculation_type, ssi.amount
             FROM salary_structure_items ssi
             JOIN salary_components sc ON sc.id = ssi.salary_component_id
             WHERE ssi.user_id = ?1 AND ssi.effective_from = ?2",
        )
        .ok()?;

    let mut breakdown = PayrollSalaryBreakdown {
        effective_from: Some(effective_from.clone()),
        source: "salary_structure_items".to_string(),
        ..Default::default()
    };

    let rows = stmt
        .query_map(rusqlite::params![user_id, &effective_from], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, f64>(5)?,
            ))
        })
        .ok()?;

    for row in rows.flatten() {
        let (comp_id, name, slug, comp_type, calc_type, amount) = row;
        let reimb = is_reimbursement_type(&comp_type, calc_type.as_deref(), &slug, &name);
        breakdown.components.push(serde_json::json!({
            "component_id": comp_id,
            "name": name,
            "slug": slug,
            "type": comp_type,
            "amount": amount,
            "is_reimbursement": reimb,
        }));

        let bucket = bucket_component(&slug, &name);
        if comp_type == "earning" {
            if reimb {
                breakdown.reimbursement += amount;
            } else {
                breakdown.gross += amount;
                match bucket {
                    "basic" => breakdown.basic += amount,
                    "hra" => breakdown.hra += amount,
                    "transport" => breakdown.transport += amount,
                    _ => breakdown.other_earnings += amount,
                }
            }
        } else if comp_type == "deduction" {
            breakdown.fixed_deductions += amount;
            match bucket {
                "pf" => breakdown.pf += amount,
                "esi" => breakdown.esi += amount,
                "tds" => breakdown.tds += amount,
                _ => breakdown.other_deductions += amount,
            }
        }
    }

    if breakdown.gross > 0.0 || breakdown.fixed_deductions > 0.0 {
        Some(breakdown)
    } else {
        None
    }
}

/// Legacy fallback: flat `salary_structures` table.
pub fn load_from_legacy_structures(
    conn: &rusqlite::Connection,
    user_id: i64,
    as_of: &str,
) -> Option<PayrollSalaryBreakdown> {
    conn.query_row(
        "SELECT basic_salary, hra, transport_allowance, other_allowances, pf_deduction, esi_deduction, tds
         FROM salary_structures WHERE user_id=?1 AND effective_from <= ?2
         ORDER BY effective_from DESC LIMIT 1",
        rusqlite::params![user_id, as_of],
        |row| {
            let basic: f64 = row.get(0)?;
            let hra: f64 = row.get(1)?;
            let transport: f64 = row.get(2)?;
            let other: f64 = row.get(3)?;
            let pf: f64 = row.get(4)?;
            let esi: f64 = row.get(5)?;
            let tds: f64 = row.get(6)?;
            let gross = basic + hra + transport + other;
            let fixed = pf + esi + tds;
            Ok(PayrollSalaryBreakdown {
                basic,
                hra,
                transport,
                other_earnings: other,
                pf,
                esi,
                tds,
                other_deductions: 0.0,
                gross,
                reimbursement: 0.0,
                fixed_deductions: fixed,
                source: "salary_structures".to_string(),
                components: vec![
                    serde_json::json!({"name": "Basic", "type": "earning", "amount": basic}),
                    serde_json::json!({"name": "HRA", "type": "earning", "amount": hra}),
                    serde_json::json!({"name": "Transport", "type": "earning", "amount": transport}),
                    serde_json::json!({"name": "Other", "type": "earning", "amount": other}),
                    serde_json::json!({"name": "PF", "type": "deduction", "amount": pf}),
                    serde_json::json!({"name": "ESI", "type": "deduction", "amount": esi}),
                    serde_json::json!({"name": "TDS", "type": "deduction", "amount": tds}),
                ],
                effective_from: None,
            })
        },
    )
    .ok()
}

pub fn load_user_salary(
    conn: &rusqlite::Connection,
    user_id: i64,
    as_of: &str,
) -> Option<PayrollSalaryBreakdown> {
    if let Some(profile) = crate::salary_split::load_employee_profile(conn, user_id, as_of) {
        let comp = crate::salary_split::load_component_split_config(conn);
        let preview = crate::salary_split::preview_for_profile(conn, &profile);
        return Some(breakdown_from_preview(&preview, &comp));
    }
    load_from_structure_items(conn, user_id, as_of)
        .or_else(|| load_from_legacy_structures(conn, user_id, as_of))
}

/// Monthly gross for analytics / reports (uses same resolution as payroll).
pub fn monthly_gross_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    as_of: &str,
) -> f64 {
    load_user_salary(conn, user_id, as_of)
        .map(|s| s.gross)
        .unwrap_or(0.0)
}

fn breakdown_from_preview(
    preview: &crate::salary_split::CtcStatutoryPreview,
    comp: &crate::salary_split::ComponentSplitConfig,
) -> PayrollSalaryBreakdown {
    let slug_for = |id: i64| -> String {
        comp.earnings
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.slug.clone())
            .unwrap_or_default()
    };
    let components: Vec<serde_json::Value> = preview
        .earning_lines
        .iter()
        .map(|line| {
            serde_json::json!({
                "component_id": line.component_id,
                "name": line.name,
                "slug": slug_for(line.component_id),
                "type": "earning",
                "amount": line.amount,
                "is_reimbursement": false,
            })
        })
        .collect();
    PayrollSalaryBreakdown {
        basic: preview.basic,
        hra: preview.hra,
        transport: preview.conveyance,
        other_earnings: preview.special,
        gross: preview.gross,
        source: preview.split_source.clone(),
        components,
        ..Default::default()
    }
}

// count_attendance_penalty_days with late arrival or early exit in the active month range.
pub fn count_attendance_penalty_days(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> i64 {
    let (active_start, active_end) =
        crate::payroll_logic::user_active_range_in_month(conn, user_id, month, year);
    if active_end < active_start {
        return 0;
    }
    let start_s = active_start.format("%Y-%m-%d").to_string();
    let end_s = active_end.format("%Y-%m-%d").to_string();

    let mut stmt = match conn.prepare(
        "SELECT DISTINCT date FROM attendance
         WHERE user_id=?1 AND date >= ?2 AND date <= ?3 AND deleted_at IS NULL
           AND clock_out IS NOT NULL AND (is_late = 1 OR is_early_exit = 1)",
    ) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    stmt.query_map(rusqlite::params![user_id, &start_s, &end_s], |row| row.get::<_, String>(0))
        .ok()
        .map(|iter| {
            iter.filter_map(|r| r.ok())
                .filter_map(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .filter(|d| crate::payroll_logic::is_working_day_for_user(conn, user_id, *d))
                .count() as i64
        })
        .unwrap_or(0)
}
