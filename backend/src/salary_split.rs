//! CTC → monthly earnings split (Basic / HRA / Conveyance / Special).

use rusqlite::Connection;
use serde::Serialize;

use crate::statutory_logic::StatutoryConfig;

#[derive(Debug, Clone)]
pub struct SplitTemplate {
    pub id: i64,
    pub name: String,
    pub basic_pct: f64,
    pub hra_pct: f64,
    pub conv_pct: f64,
    pub special_pct: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CtcSplit {
    pub yearly_ctc: f64,
    pub monthly_ctc: f64,
    pub basic: f64,
    pub hra: f64,
    pub conveyance: f64,
    pub special: f64,
    pub gross: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentLine {
    pub component_id: i64,
    pub name: String,
    pub pct: f64,
    pub amount: f64,
    #[serde(default)]
    pub is_employer: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CtcStatutoryPreview {
    pub yearly_ctc: f64,
    pub monthly_ctc: f64,
    pub employer_pf: f64,
    pub employer_esi: f64,
    pub employer_lw: f64,
    pub total_employer: f64,
    pub gross: f64,
    pub basic: f64,
    pub hra: f64,
    pub conveyance: f64,
    pub special: f64,
    pub basic_pct: f64,
    pub hra_pct: f64,
    pub conv_pct: f64,
    pub special_pct: f64,
    pub employee_pf: f64,
    pub employee_esi: f64,
    pub employee_lw: f64,
    pub prof_tax: f64,
    pub employee_tds: f64,
    pub total_employee_deductions: f64,
    pub net_take_home: f64,
    pub pf_applicable: bool,
    pub esi_applicable: bool,
    pub esi_applies: bool,
    pub esi_note: Option<String>,
    pub earning_lines: Vec<ComponentLine>,
    pub deduction_lines: Vec<ComponentLine>,
    pub split_source: String,
}

impl CtcStatutoryPreview {
    pub fn to_ctc_split(&self) -> CtcSplit {
        CtcSplit {
            yearly_ctc: self.yearly_ctc,
            monthly_ctc: self.monthly_ctc,
            basic: self.basic,
            hra: self.hra,
            conveyance: self.conveyance,
            special: self.special,
            gross: self.gross,
        }
    }
}

fn employer_statutory(
    cfg: &StatutoryConfig,
    gross: f64,
    basic: f64,
    pf_applicable: bool,
    esi_applicable: bool,
) -> (f64, f64, f64, f64) {
    let mut pf = 0.0;
    let mut esi = 0.0;
    let lw = if pf_applicable || esi_applicable {
        cfg.lw_employer
    } else {
        0.0
    };
    if pf_applicable && basic > 0.0 {
        pf = round2(basic.min(cfg.pf_wage_ceiling) * cfg.pf_employer_rate);
    }
    if esi_applicable && gross > 0.0 && gross <= cfg.esi_gross_ceiling {
        esi = round2(gross * cfg.esi_employer_rate);
    }
    (pf, esi, round2(lw), round2(pf + esi + lw))
}

fn is_employer_deduction(d: &LoadedDeduction) -> bool {
    let s = d.slug.to_lowercase();
    let n = d.name.to_lowercase();
    s.contains("employer") || n.contains("employer")
}

/// Resolve monthly deduction amount from a salary component definition.
pub fn resolve_deduction_amount(
    d: &LoadedDeduction,
    gross: f64,
    basic: f64,
    cfg: &StatutoryConfig,
) -> Option<f64> {
    let slug = d.slug.to_lowercase();
    let name = d.name.to_lowercase();
    let employer = is_employer_deduction(d);

    if slug.contains("pf") || name.contains("provident") {
        let base = basic.min(cfg.pf_wage_ceiling);
        if d.calc_type.contains("percentage") && d.amount > 0.0 {
            return Some(round2(base * d.amount / 100.0));
        }
        if d.amount > 0.0 && d.calc_type == "flat_amount" {
            return Some(round2(d.amount));
        }
        let rate = if employer {
            cfg.pf_employer_rate
        } else {
            cfg.pf_employee_rate
        };
        return Some(round2(base * rate));
    }

    if slug.contains("esi") || name.contains("esi") {
        if gross > cfg.esi_gross_ceiling {
            return None;
        }
        if d.calc_type.contains("percentage") && d.amount > 0.0 {
            return Some(round2(gross * d.amount / 100.0));
        }
        if d.amount > 0.0 && d.calc_type == "flat_amount" {
            return Some(round2(d.amount));
        }
        let rate = if employer {
            cfg.esi_employer_rate
        } else {
            cfg.esi_employee_rate
        };
        return Some(round2(gross * rate));
    }

    if slug.contains("professional") || slug.contains("prof_tax") || name.contains("professional tax") {
        if d.amount > 0.0 {
            return Some(round2(d.amount));
        }
        return Some(round2(cfg.prof_tax_default));
    }

    if slug.contains("labour") || slug.contains("welfare") || slug.contains("lw") {
        if d.amount > 0.0 {
            return Some(round2(d.amount));
        }
        let default = if employer {
            cfg.lw_employer
        } else {
            cfg.lw_employee
        };
        return Some(round2(default));
    }

    if slug.contains("tds") || name.contains("tax deducted") {
        return if d.amount > 0.0 {
            Some(round2(d.amount))
        } else {
            None
        };
    }

    match d.calc_type.as_str() {
        "percentage_of_gross" | "percentage_of_ctc" => Some(round2(gross * d.amount / 100.0)),
        "percentage_of_basic" => Some(round2(basic * d.amount / 100.0)),
        _ => {
            if d.amount > 0.0 {
                Some(round2(d.amount))
            } else {
                None
            }
        }
    }
}

fn deduction_component_applicable(d: &LoadedDeduction, pf_on: bool, esi_on: bool) -> bool {
    let slug = d.slug.to_lowercase();
    let name = d.name.to_lowercase();
    if slug.contains("pf") || name.contains("provident") {
        return pf_on;
    }
    if slug.contains("esi") || name.contains("esi") {
        return esi_on;
    }
    true
}

/// Employee-side deduction lines for payroll (from salary_components, after LOP).
pub fn build_payroll_deduction_lines(
    comp: &ComponentSplitConfig,
    cfg: &StatutoryConfig,
    basic: f64,
    gross: f64,
    advance: f64,
    pf_applicable: bool,
    esi_applicable: bool,
) -> Vec<ComponentLine> {
    let pf_on = pf_applicable && comp.has_pf;
    let esi_on = esi_applicable && comp.has_esi;
    let mut lines = Vec::new();
    for d in &comp.deductions {
        if is_employer_deduction(d) {
            continue;
        }
        if !deduction_component_applicable(d, pf_on, esi_on) {
            continue;
        }
        let Some(amt) = resolve_deduction_amount(d, gross, basic, cfg) else {
            continue;
        };
        if amt <= 0.0 {
            continue;
        }
        lines.push(ComponentLine {
            component_id: d.id,
            name: d.name.clone(),
            pct: if d.calc_type.contains("percentage") {
                d.amount
            } else {
                0.0
            },
            amount: amt,
            is_employer: false,
        });
    }
    if advance > 0.0 {
        lines.push(ComponentLine {
            component_id: 0,
            name: "Advance Recovery".into(),
            pct: 0.0,
            amount: round2(advance),
            is_employer: false,
        });
    }
    lines
}

/// Sum employee deduction lines into a statutory result (single source for payroll totals).
pub fn statutory_result_from_lines(lines: &[ComponentLine]) -> crate::statutory_logic::StatutoryResult {
    let mut r = crate::statutory_logic::StatutoryResult::default();
    for line in lines {
        let n = line.name.to_lowercase();
        if line.name == "Advance Recovery" {
            r.advance = round2(r.advance + line.amount);
        } else if n.contains("provident") || n.contains(" pf") || n.starts_with("pf") {
            r.pf_employee = round2(r.pf_employee + line.amount);
        } else if n.contains("esi") || n.contains("state insurance") {
            r.esi_employee = round2(r.esi_employee + line.amount);
        } else if n.contains("professional") {
            r.prof_tax = round2(r.prof_tax + line.amount);
        } else if n.contains("labour") || n.contains("welfare") {
            r.lw_employee = round2(r.lw_employee + line.amount);
        } else if n.contains("tax deducted") || n.contains("tds") {
            r.other_deductions = round2(r.other_deductions + line.amount);
        } else {
            r.other_deductions = round2(r.other_deductions + line.amount);
        }
    }
    r.total_employee = round2(
        r.pf_employee + r.esi_employee + r.prof_tax + r.lw_employee + r.advance + r.other_deductions,
    );
    r
}

#[derive(Debug, Clone)]
pub struct LoadedEarning {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub calc_type: String,
    pub pct: f64,
    pub flat: f64,
}

#[derive(Debug, Clone)]
pub struct LoadedDeduction {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub calc_type: String,
    pub amount: f64,
    pub is_pre_tax: bool,
}

#[derive(Debug, Clone)]
pub struct ComponentSplitConfig {
    pub earnings: Vec<LoadedEarning>,
    pub deductions: Vec<LoadedDeduction>,
    pub basic_pct: f64,
    pub hra_pct: f64,
    pub conv_pct: f64,
    pub special_pct: f64,
    pub pct_sum: f64,
    pub has_pf: bool,
    pub has_esi: bool,
    pub has_prof_tax: bool,
    pub has_tds: bool,
    pub prof_tax_flat: f64,
    pub tds_flat: f64,
}

fn bucket_pct(name: &str, slug: &str, pct: f64) -> (f64, f64, f64, f64) {
    let n = name.to_lowercase();
    let s = slug.to_lowercase();
    if s.contains("basic") || n.contains("basic") {
        (pct, 0.0, 0.0, 0.0)
    } else if s.contains("hra") || n.contains("house rent") || n.contains("hra") {
        (0.0, pct, 0.0, 0.0)
    } else if s.contains("travel") || s.contains("conveyance") || n.contains("travel") || n.contains("conveyance") {
        (0.0, 0.0, pct, 0.0)
    } else if s.contains("special") || n.contains("special") {
        (0.0, 0.0, 0.0, pct)
    } else {
        (0.0, 0.0, 0.0, pct)
    }
}

/// Load active salary component definitions — single source of truth for CTC split.
pub fn load_component_split_config(conn: &Connection) -> ComponentSplitConfig {
    let mut earnings = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, name, COALESCE(slug,'') AS slug, COALESCE(calculation_type,'flat_amount') AS calc_type,
                COALESCE(default_value, amount) AS val, is_active
         FROM salary_components
         WHERE COALESCE(component_type, type)='earning'
           AND COALESCE(is_active, 1) != 0
         ORDER BY id",
    ) {
        let rows = stmt
            .query_map([], |row| {
                let calc: String = row.get(3)?;
                let val: Option<f64> = row.get(4)?;
                let pct = if calc.contains("percentage") {
                    val.unwrap_or(0.0)
                } else {
                    0.0
                };
                let flat = if calc == "flat_amount" {
                    val.unwrap_or(0.0)
                } else {
                    0.0
                };
                Ok(LoadedEarning {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slug: row.get(2)?,
                    calc_type: calc,
                    pct,
                    flat,
                })
            })
            .ok();
        if let Some(rows) = rows {
            earnings = rows.filter_map(|r| r.ok()).collect();
        }
    }

    let mut deductions = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, name, COALESCE(slug, LOWER(REPLACE(name,' ','_'))) AS slug,
                COALESCE(calculation_type,'flat_amount') AS calc_type,
                COALESCE(default_value, amount, 0) AS val,
                COALESCE(is_pre_tax, 0) AS is_pre_tax
         FROM salary_components
         WHERE COALESCE(component_type, type)='deduction'
           AND COALESCE(is_active, 1) != 0
         ORDER BY id",
    ) {
        let rows = stmt
            .query_map([], |row| {
                Ok(LoadedDeduction {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slug: row.get(2)?,
                    calc_type: row.get(3)?,
                    amount: row.get(4)?,
                    is_pre_tax: row.get::<_, i64>(5).unwrap_or(0) != 0,
                })
            })
            .ok();
        if let Some(rows) = rows {
            deductions = rows.filter_map(|r| r.ok()).collect();
        }
    }

    let mut basic_pct = 0.0;
    let mut hra_pct = 0.0;
    let mut conv_pct = 0.0;
    let mut special_pct = 0.0;
    let mut pct_sum = 0.0;
    for e in &earnings {
        if e.calc_type.contains("percentage") {
            pct_sum += e.pct;
            let (b, h, c, s) = bucket_pct(&e.name, &e.slug, e.pct);
            basic_pct += b;
            hra_pct += h;
            conv_pct += c;
            special_pct += s;
        }
    }
    if pct_sum <= 0.0 {
        basic_pct = 50.0;
        hra_pct = 35.0;
        conv_pct = 15.0;
        pct_sum = 100.0;
    }

    let slug_match = |d: &LoadedDeduction, keys: &[&str]| {
        keys.iter().any(|k| d.slug.contains(k) || d.name.to_lowercase().contains(k))
    };
    let has_pf = deductions.iter().any(|d| slug_match(d, &["pf", "provident"]));
    let has_esi = deductions.iter().any(|d| slug_match(d, &["esi"]));
    let has_prof_tax = deductions.iter().any(|d| slug_match(d, &["professional_tax", "prof_tax", "professional tax"]));
    let has_tds = deductions.iter().any(|d| slug_match(d, &["tds", "tax deducted"]));
    let prof_tax_flat = deductions
        .iter()
        .find(|d| slug_match(d, &["professional_tax", "prof_tax", "professional tax"]))
        .map(|d| d.amount)
        .unwrap_or(0.0);
    let tds_flat = deductions
        .iter()
        .find(|d| slug_match(d, &["tds", "tax deducted"]))
        .map(|d| d.amount)
        .unwrap_or(0.0);

    ComponentSplitConfig {
        earnings,
        deductions,
        basic_pct,
        hra_pct,
        conv_pct,
        special_pct,
        pct_sum,
        has_pf,
        has_esi,
        has_prof_tax,
        has_tds,
        prof_tax_flat,
        tds_flat,
    }
}

pub fn split_with_statutory_from_components(
    comp: &ComponentSplitConfig,
    yearly_ctc: f64,
    cfg: &StatutoryConfig,
    pf_applicable: bool,
    esi_applicable: bool,
) -> CtcStatutoryPreview {
    let pf_on = pf_applicable && comp.has_pf;
    let esi_on = esi_applicable && comp.has_esi;
    let monthly_ctc = round2(yearly_ctc / 12.0);
    let pct_sum = comp.pct_sum.max(1.0);
    let gross_base = monthly_ctc;

    let mut earning_lines = Vec::new();
    let mut basic = 0.0;
    let mut hra = 0.0;
    let mut conveyance = 0.0;
    let mut special = 0.0;

    for e in &comp.earnings {
        let amt = if e.calc_type.contains("percentage") {
            round2(gross_base * e.pct / pct_sum)
        } else {
            round2(e.flat)
        };
        if amt <= 0.0 {
            continue;
        }
        let n = e.name.to_lowercase();
        let s = e.slug.to_lowercase();
        if s.contains("basic") || n.contains("basic") {
            basic = amt;
        } else if s.contains("hra") || n.contains("house rent") || n.contains("hra") {
            hra = amt;
        } else if s.contains("travel") || s.contains("conveyance") || n.contains("travel") || n.contains("conveyance") {
            conveyance = amt;
        } else {
            special += amt;
        }
        earning_lines.push(ComponentLine {
            component_id: e.id,
            name: e.name.clone(),
            pct: e.pct,
            amount: amt,
            is_employer: false,
        });
    }
    let gross = round2(basic + hra + conveyance + special);

    let esi_applies = esi_on && gross > 0.0 && gross <= cfg.esi_gross_ceiling;
    let esi_note = if esi_on && gross > cfg.esi_gross_ceiling {
        Some(format!(
            "ESI not applicable — gross exceeds ceiling of ₹{}",
            cfg.esi_gross_ceiling as i64
        ))
    } else {
        None
    };

    let mut deduction_lines = Vec::new();
    let mut employee_pf = 0.0;
    let mut employee_esi = 0.0;
    let mut employee_lw = 0.0;
    let mut prof_tax = 0.0;
    let mut employee_tds = 0.0;
    let mut total_employee_deductions = 0.0;

    for d in &comp.deductions {
        let employer = is_employer_deduction(d);
        if !employer && !deduction_component_applicable(d, pf_on, esi_on) {
            continue;
        }
        let Some(amt) = resolve_deduction_amount(d, gross, basic, cfg) else {
            continue;
        };
        if amt <= 0.0 {
            continue;
        }
        if !employer {
            total_employee_deductions = round2(total_employee_deductions + amt);
        }
        deduction_lines.push(ComponentLine {
            component_id: d.id,
            name: d.name.clone(),
            pct: if d.calc_type.contains("percentage") {
                d.amount
            } else {
                0.0
            },
            amount: amt,
            is_employer: employer,
        });

        let slug = d.slug.to_lowercase();
        let name = d.name.to_lowercase();
        if !employer && (slug.contains("pf") || name.contains("provident")) {
            employee_pf = amt;
        } else if !employer && slug.contains("esi") {
            employee_esi = amt;
        } else if !employer
            && (slug.contains("labour") || slug.contains("welfare") || slug.contains("lw"))
        {
            employee_lw = amt;
        } else if slug.contains("professional") || slug.contains("prof_tax") {
            prof_tax = amt;
        } else if slug.contains("tds") {
            employee_tds = amt;
        }
    }

    let net_take_home = round2((gross - total_employee_deductions).max(0.0));

    CtcStatutoryPreview {
        yearly_ctc,
        monthly_ctc,
        employer_pf: 0.0,
        employer_esi: 0.0,
        employer_lw: 0.0,
        total_employer: 0.0,
        gross,
        basic,
        hra,
        conveyance,
        special,
        basic_pct: comp.basic_pct,
        hra_pct: comp.hra_pct,
        conv_pct: comp.conv_pct,
        special_pct: comp.special_pct,
        employee_pf,
        employee_esi,
        employee_lw,
        prof_tax,
        employee_tds,
        total_employee_deductions,
        net_take_home,
        pf_applicable: pf_on,
        esi_applicable: esi_on,
        esi_applies,
        esi_note,
        earning_lines,
        deduction_lines,
        split_source: "salary_components".into(),
    }
}

impl SplitTemplate {
    pub fn default_standard() -> Self {
        Self {
            id: 0,
            name: "Standard (50/35/15)".to_string(),
            basic_pct: 50.0,
            hra_pct: 35.0,
            conv_pct: 15.0,
            special_pct: 0.0,
        }
    }

    /// Legacy: splits full CTC into components with no employer statutory carve-out.
    pub fn split(&self, yearly_ctc: f64) -> CtcSplit {
        self.split_with_statutory(yearly_ctc, false, false, &StatutoryConfig::default())
            .to_ctc_split()
    }

    /// Indian CTC model: Monthly CTC = Gross + Employer PF + Employer ESI + Employer LW.
    /// Template percentages apply to Gross Pay.
    pub fn split_with_statutory(
        &self,
        yearly_ctc: f64,
        pf_applicable: bool,
        esi_applicable: bool,
        cfg: &StatutoryConfig,
    ) -> CtcStatutoryPreview {
        let monthly_ctc = round2(yearly_ctc / 12.0);
        let pct_sum = (self.basic_pct + self.hra_pct + self.conv_pct + self.special_pct).max(1.0);

        let mut gross = monthly_ctc;
        if pf_applicable || esi_applicable {
            for _ in 0..48 {
                let basic_est = round2(gross * self.basic_pct / pct_sum);
                let (_, _, _, employer_total) =
                    employer_statutory(cfg, gross, basic_est, pf_applicable, esi_applicable);
                let next = round2((monthly_ctc - employer_total).max(0.0));
                if (next - gross).abs() < 0.01 {
                    gross = next;
                    break;
                }
                gross = next;
            }
        }

        let basic = round2(gross * self.basic_pct / pct_sum);
        let hra = round2(gross * self.hra_pct / pct_sum);
        let conveyance = round2(gross * self.conv_pct / pct_sum);
        let special = round2(gross * self.special_pct / pct_sum);
        let gross = round2(basic + hra + conveyance + special);

        let (employer_pf, employer_esi, employer_lw, total_employer) =
            employer_statutory(cfg, gross, basic, pf_applicable, esi_applicable);

        let mut employee_pf = 0.0;
        if pf_applicable && basic > 0.0 {
            employee_pf = round2(basic.min(cfg.pf_wage_ceiling) * cfg.pf_employee_rate);
        }

        let esi_applies = esi_applicable && gross > 0.0 && gross <= cfg.esi_gross_ceiling;
        let esi_note = if esi_applicable && gross > cfg.esi_gross_ceiling {
            Some(format!(
                "ESI not applicable — gross exceeds ceiling of ₹{}",
                cfg.esi_gross_ceiling as i64
            ))
        } else {
            None
        };
        let employee_esi = if esi_applies {
            round2(gross * cfg.esi_employee_rate)
        } else {
            0.0
        };

        let employee_lw = if pf_applicable || esi_applicable {
            round2(cfg.lw_employee)
        } else {
            0.0
        };
        let prof_tax = round2(cfg.prof_tax_default);
        let total_employee_deductions = round2(employee_pf + employee_esi + employee_lw + prof_tax);
        let net_take_home = round2((gross - total_employee_deductions).max(0.0));

        CtcStatutoryPreview {
            yearly_ctc,
            monthly_ctc,
            employer_pf,
            employer_esi,
            employer_lw,
            total_employer,
            gross,
            basic,
            hra,
            conveyance,
            special,
            basic_pct: self.basic_pct,
            hra_pct: self.hra_pct,
            conv_pct: self.conv_pct,
            special_pct: self.special_pct,
            employee_pf,
            employee_esi,
            employee_lw,
            prof_tax,
            employee_tds: 0.0,
            total_employee_deductions,
            net_take_home,
            pf_applicable,
            esi_applicable,
            esi_applies,
            esi_note,
            earning_lines: vec![],
            deduction_lines: vec![],
            split_source: "template".into(),
        }
    }
}

pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn load_template(conn: &Connection, template_id: i64) -> Option<SplitTemplate> {
    conn.query_row(
        "SELECT id, name, basic_pct, hra_pct, conv_pct, special_pct FROM salary_templates WHERE id=?1",
        [template_id],
        |row| {
            Ok(SplitTemplate {
                id: row.get(0)?,
                name: row.get(1)?,
                basic_pct: row.get(2)?,
                hra_pct: row.get(3)?,
                conv_pct: row.get(4)?,
                special_pct: row.get(5)?,
            })
        },
    )
    .ok()
}

pub fn load_default_template(conn: &Connection) -> SplitTemplate {
    load_template(
        conn,
        conn.query_row(
            "SELECT id FROM salary_templates WHERE is_default=1 ORDER BY id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(1),
    )
    .unwrap_or_else(SplitTemplate::default_standard)
}

#[derive(Debug, Clone)]
pub struct EmployeeSalaryProfile {
    pub user_id: i64,
    pub yearly_ctc: f64,
    pub template_id: Option<i64>,
    pub pf_applicable: bool,
    pub esi_applicable: bool,
    pub effective_from: String,
}

pub fn load_employee_profile(
    conn: &Connection,
    user_id: i64,
    as_of: &str,
) -> Option<EmployeeSalaryProfile> {
    conn.query_row(
        "SELECT user_id, yearly_ctc, template_id, pf_applicable, esi_applicable, effective_from
         FROM employee_salary_profiles
         WHERE user_id=?1 AND effective_from <= ?2
         ORDER BY effective_from DESC LIMIT 1",
        rusqlite::params![user_id, as_of],
        |row| {
            Ok(EmployeeSalaryProfile {
                user_id: row.get(0)?,
                yearly_ctc: row.get(1)?,
                template_id: row.get(2)?,
                pf_applicable: row.get::<_, i64>(3).unwrap_or(1) != 0,
                esi_applicable: row.get::<_, i64>(4).unwrap_or(1) != 0,
                effective_from: row.get(5)?,
            })
        },
    )
    .ok()
    .filter(|p| p.yearly_ctc > 0.0)
}

pub fn preview_from_inputs(conn: &Connection, yearly_ctc: f64) -> CtcStatutoryPreview {
    let comp = load_component_split_config(conn);
    let cfg = crate::statutory_logic::load_statutory_config(conn);
    split_with_statutory_from_components(&comp, yearly_ctc, &cfg, comp.has_pf, comp.has_esi)
}

pub fn preview_for_profile(conn: &Connection, profile: &EmployeeSalaryProfile) -> CtcStatutoryPreview {
    let comp = load_component_split_config(conn);
    let cfg = crate::statutory_logic::load_statutory_config(conn);
    split_with_statutory_from_components(
        &comp,
        profile.yearly_ctc,
        &cfg,
        profile.pf_applicable,
        profile.esi_applicable,
    )
}

pub fn split_for_user(conn: &Connection, user_id: i64, as_of: &str) -> Option<(EmployeeSalaryProfile, CtcSplit)> {
    let profile = load_employee_profile(conn, user_id, as_of)?;
    let preview = preview_for_profile(conn, &profile);
    Some((profile, preview.to_ctc_split()))
}

/// Map preview amounts onto salary_structure_items using component IDs from salary_components.
pub fn sync_structure_from_preview(
    conn: &Connection,
    user_id: i64,
    effective_from: &str,
    preview: &CtcStatutoryPreview,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "DELETE FROM salary_structure_items WHERE user_id=?1",
        [user_id],
    )?;
    for line in &preview.earning_lines {
        if line.amount <= 0.0 || line.component_id <= 0 {
            continue;
        }
        conn.execute(
            "INSERT INTO salary_structure_items (user_id, salary_component_id, amount, effective_from, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?5)",
            rusqlite::params![user_id, line.component_id, line.amount, effective_from, &now],
        )?;
    }
    Ok(())
}

/// Legacy wrapper — prefer sync_structure_from_preview.
pub fn sync_structure_from_ctc(
    conn: &Connection,
    user_id: i64,
    effective_from: &str,
    split: &CtcSplit,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mappings: [(&str, &[&str], f64); 4] = [
        ("basic", &["basic"], split.basic),
        ("hra", &["hra", "house rent"], split.hra),
        ("conveyance", &["conveyance", "travel"], split.conveyance),
        ("special", &["special"], split.special),
    ];

    conn.execute(
        "DELETE FROM salary_structure_items WHERE user_id=?1 AND effective_from=?2",
        rusqlite::params![user_id, effective_from],
    )?;

    for (_key, patterns, amount) in mappings {
        if amount <= 0.0 {
            continue;
        }
        let comp_id = find_earning_component(conn, patterns);
        if let Some(cid) = comp_id {
            conn.execute(
                "INSERT INTO salary_structure_items (user_id, salary_component_id, amount, effective_from, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?5)",
                rusqlite::params![user_id, cid, amount, effective_from, &now],
            )?;
        }
    }
    Ok(())
}

fn find_earning_component(conn: &Connection, patterns: &[&str]) -> Option<i64> {
    for pat in patterns {
        let like = format!("%{}%", pat.to_lowercase());
        if let Ok(id) = conn.query_row(
            "SELECT id FROM salary_components
             WHERE COALESCE(component_type, type)='earning'
               AND (LOWER(COALESCE(slug,'')) LIKE ?1 OR LOWER(name) LIKE ?1)
             ORDER BY id LIMIT 1",
            [&like],
            |r| r.get(0),
        ) {
            return Some(id);
        }
    }
    None
}

/// Force-update standard earning rows to match a CTC template (idempotent, runs every startup).
pub fn sync_component_definitions_from_template(conn: &Connection, tpl: &SplitTemplate) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let defs: [(&[&str], f64); 4] = [
        (&["basic pay", "basic"], tpl.basic_pct),
        (&["house rent", "hra"], tpl.hra_pct),
        (&["travel", "conveyance"], tpl.conv_pct),
        (&["special"], tpl.special_pct),
    ];
    let mut updated = 0usize;
    for (patterns, pct) in defs {
        if let Some(id) = find_earning_component(conn, patterns) {
            let n =             conn.execute(
                "UPDATE salary_components SET
                    calculation_type='percentage_of_gross',
                    amount=?1, default_value=?1,
                    updated_at=?2
                 WHERE id=?3",
                rusqlite::params![pct, &now, id],
            )?;
            if n > 0 {
                updated += 1;
            }
        }
    }
    log::info!(
        "CTC component sync: updated {updated} earning components to template {} ({}% / {}% / {}% / {}%)",
        tpl.name, tpl.basic_pct, tpl.hra_pct, tpl.conv_pct, tpl.special_pct
    );
    Ok(())
}

pub fn amount_from_calc(
    calc_type: &str,
    pct_or_amount: f64,
    monthly_ctc: f64,
    gross_amount: f64,
    basic_amount: f64,
) -> f64 {
    match calc_type {
        "percentage_of_ctc" => round2(monthly_ctc * pct_or_amount / 100.0),
        "percentage_of_gross" => round2(gross_amount * pct_or_amount / 100.0),
        "percentage_of_basic" => round2(basic_amount * pct_or_amount / 100.0),
        _ => pct_or_amount,
    }
}
