//! PF, ESI, Professional Tax, Labour Welfare calculations.

use rusqlite::Connection;

use crate::salary_split::round2;

#[derive(Debug, Clone)]
pub struct StatutoryConfig {
    pub pf_wage_ceiling: f64,
    pub pf_employee_rate: f64,
    pub pf_employer_rate: f64,
    pub esi_gross_ceiling: f64,
    pub esi_employee_rate: f64,
    pub esi_employer_rate: f64,
    pub esi_admin_rate: f64,
    pub prof_tax_default: f64,
    pub lw_employee: f64,
    pub lw_employer: f64,
}

impl Default for StatutoryConfig {
    fn default() -> Self {
        Self {
            pf_wage_ceiling: 15_000.0,
            pf_employee_rate: 0.12,
            pf_employer_rate: 0.12,
            esi_gross_ceiling: 21_000.0,
            esi_employee_rate: 0.0075,
            esi_employer_rate: 0.0325,
            esi_admin_rate: 0.0,
            prof_tax_default: 200.0,
            lw_employee: 50.0,
            lw_employer: 50.0,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct StatutoryResult {
    pub epf_wages: f64,
    pub pf_employee: f64,
    pub pf_employer: f64,
    pub esi_wages: f64,
    pub esi_employee: f64,
    pub esi_employer: f64,
    pub esi_admin: f64,
    pub prof_tax: f64,
    pub lw_employee: f64,
    pub lw_employer: f64,
    pub advance: f64,
    pub other_deductions: f64,
    pub total_employee: f64,
    pub total_employer: f64,
}

pub fn load_statutory_config(conn: &Connection) -> StatutoryConfig {
    let mut cfg = StatutoryConfig::default();
    let keys = [
        ("pf_wage_ceiling", &mut cfg.pf_wage_ceiling),
        ("pf_employee_rate", &mut cfg.pf_employee_rate),
        ("pf_employer_rate", &mut cfg.pf_employer_rate),
        ("esi_gross_ceiling", &mut cfg.esi_gross_ceiling),
        ("esi_employee_rate", &mut cfg.esi_employee_rate),
        ("esi_employer_rate", &mut cfg.esi_employer_rate),
        ("esi_admin_rate", &mut cfg.esi_admin_rate),
        ("prof_tax_default", &mut cfg.prof_tax_default),
        ("lw_employee", &mut cfg.lw_employee),
        ("lw_employer", &mut cfg.lw_employer),
    ];
    for (key, slot) in keys {
        if let Ok(v) = conn.query_row(
            "SELECT value FROM app_settings WHERE key=?1",
            [key],
            |r| r.get::<_, String>(0),
        ) {
            if let Ok(parsed) = v.parse::<f64>() {
                *slot = parsed;
            }
        }
    }
    cfg
}

pub fn calculate_statutory(
    cfg: &StatutoryConfig,
    basic_after_lop: f64,
    gross_after_lop: f64,
    pf_applicable: bool,
    esi_applicable: bool,
    advance: f64,
    other: f64,
) -> StatutoryResult {
    let mut r = StatutoryResult {
        advance,
        other_deductions: other,
        ..Default::default()
    };

    if pf_applicable && basic_after_lop > 0.0 {
        r.epf_wages = round2(basic_after_lop.min(cfg.pf_wage_ceiling));
        r.pf_employee = round2(r.epf_wages * cfg.pf_employee_rate);
        r.pf_employer = round2(r.epf_wages * cfg.pf_employer_rate);
    }

    if esi_applicable && gross_after_lop > 0.0 && gross_after_lop <= cfg.esi_gross_ceiling {
        r.esi_wages = round2(gross_after_lop);
        r.esi_employee = round2(r.esi_wages * cfg.esi_employee_rate);
        r.esi_employer = round2(r.esi_wages * cfg.esi_employer_rate);
        r.esi_admin = round2(r.esi_wages * cfg.esi_admin_rate);
    }

    r.prof_tax = round2(cfg.prof_tax_default);
    r.lw_employee = round2(cfg.lw_employee);
    r.lw_employer = round2(cfg.lw_employer);

    r.total_employee = round2(
        r.pf_employee + r.esi_employee + r.prof_tax + r.lw_employee + r.advance + r.other_deductions,
    );
    r.total_employer = round2(r.pf_employer + r.esi_employer + r.esi_admin + r.lw_employer);
    r
}

pub fn advance_emi_for_month(conn: &Connection, user_id: i64) -> f64 {
    conn.query_row(
        "SELECT COALESCE(SUM(monthly_emi), 0) FROM employee_advances
         WHERE user_id=?1 AND is_active=1 AND balance > 0",
        [user_id],
        |r| r.get(0),
    )
    .unwrap_or(0.0)
}
