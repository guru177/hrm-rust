//! Shared payroll calculations: business days, LOP, adjustments.

use chrono::{Datelike, NaiveDate, Weekday};
use std::collections::{HashMap, HashSet};

pub fn is_weekend(d: NaiveDate) -> bool {
    matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

pub fn is_business_day(d: NaiveDate) -> bool {
    !is_weekend(d)
}

pub fn is_working_day_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    d: NaiveDate,
) -> bool {
    let date_str = d.format("%Y-%m-%d").to_string();
    crate::shift_logic::user_is_scheduled_working_day(conn, user_id, &date_str, d)
}

pub fn month_bounds(month: i32, year: i32) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap();
    let end = start
        .with_day(start.num_days_in_month().into())
        .unwrap_or(start);
    (start, end)
}

pub fn calendar_days_in_month(month: i32, year: i32) -> i64 {
    NaiveDate::from_ymd_opt(year, month as u32, 1)
        .map(|d| d.num_days_in_month() as i64)
        .unwrap_or(30)
}

pub fn business_days_between(start: NaiveDate, end: NaiveDate) -> i64 {
    if end < start {
        return 0;
    }
    let mut count = 0i64;
    let mut d = start;
    while d <= end {
        if is_business_day(d) {
            count += 1;
        }
        d += chrono::Duration::days(1);
    }
    count
}

pub fn working_days_between_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    start: NaiveDate,
    end: NaiveDate,
) -> i64 {
    if end < start {
        return 0;
    }
    let mut count = 0i64;
    let mut d = start;
    while d <= end {
        if is_working_day_for_user(conn, user_id, d) {
            count += 1;
        }
        d += chrono::Duration::days(1);
    }
    count
}

fn business_dates_between(start: NaiveDate, end: NaiveDate) -> HashSet<NaiveDate> {
    let mut dates = HashSet::new();
    if end < start {
        return dates;
    }
    let mut d = start;
    while d <= end {
        if is_business_day(d) {
            dates.insert(d);
        }
        d += chrono::Duration::days(1);
    }
    dates
}

fn working_dates_between_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    start: NaiveDate,
    end: NaiveDate,
) -> HashSet<NaiveDate> {
    let mut dates = HashSet::new();
    if end < start {
        return dates;
    }
    let mut d = start;
    while d <= end {
        if is_working_day_for_user(conn, user_id, d) {
            dates.insert(d);
        }
        d += chrono::Duration::days(1);
    }
    dates
}

/// Returns active date range for user within a payroll month (join/exit clipped).
pub fn user_active_range_in_month(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> (NaiveDate, NaiveDate) {
    let (month_start, month_end) = month_bounds(month, year);
    let (join, exit): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT date_of_joining, date_of_exit FROM users WHERE id=?1",
            [user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((None, None));

    let mut range_start = month_start;
    let mut range_end = month_end;

    if let Some(ref j) = join {
        if let Ok(d) = NaiveDate::parse_from_str(j, "%Y-%m-%d") {
            if d > range_start {
                range_start = d;
            }
        }
    }
    if let Some(ref e) = exit {
        if !e.is_empty() {
            if let Ok(d) = NaiveDate::parse_from_str(e, "%Y-%m-%d") {
                if d < range_end {
                    range_end = d;
                }
            }
        }
    }
    (range_start, range_end)
}

/// Business days in month, optionally clipped to employee active range.
pub fn working_days_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> i64 {
    let (range_start, range_end) = user_active_range_in_month(conn, user_id, month, year);
    if range_end < range_start {
        return 0;
    }
    working_days_between_for_user(conn, user_id, range_start, range_end)
}

pub fn count_holidays_on_business_days(
    conn: &rusqlite::Connection,
    month: i32,
    year: i32,
) -> i64 {
    let (start, end) = month_bounds(month, year);
    let start_s = start.format("%Y-%m-%d").to_string();
    let end_s = end.format("%Y-%m-%d").to_string();

    let mut stmt = match conn.prepare(
        "SELECT date FROM holidays WHERE date >= ?1 AND date <= ?2",
    ) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    stmt.query_map(rusqlite::params![start_s, end_s], |row| row.get::<_, String>(0))
        .ok()
        .map(|iter| {
            iter.filter_map(|r| r.ok())
                .filter_map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .filter(|d| is_business_day(*d))
                .count() as i64
        })
        .unwrap_or(0)
}

pub fn business_days_in_leave_overlap(
    conn: &rusqlite::Connection,
    user_id: i64,
    leave_start: NaiveDate,
    leave_end: NaiveDate,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> i64 {
    let overlap_start = leave_start.max(range_start);
    let overlap_end = leave_end.min(range_end);
    if overlap_end < overlap_start {
        return 0;
    }
    working_days_between_for_user(conn, user_id, overlap_start, overlap_end)
}

/// Business-day holidays that fall within the user's active range in the month.
pub fn paid_holidays_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> i64 {
    let (range_start, range_end) = user_active_range_in_month(conn, user_id, month, year);
    if range_end < range_start {
        return 0;
    }
    let start_s = month_bounds(month, year).0.format("%Y-%m-%d").to_string();
    let end_s = month_bounds(month, year).1.format("%Y-%m-%d").to_string();

    let mut stmt = match conn.prepare(
        "SELECT date FROM holidays WHERE date >= ?1 AND date <= ?2",
    ) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    stmt.query_map(rusqlite::params![start_s, end_s], |row| row.get::<_, String>(0))
        .ok()
        .map(|iter| {
            iter.filter_map(|r| r.ok())
                .filter_map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .filter(|d| {
                    *d >= range_start
                        && *d <= range_end
                        && is_working_day_for_user(conn, user_id, *d)
                })
                .count() as i64
        })
        .unwrap_or(0)
}

/// Distinct business-day leave dates for a user within a range and status filter.
pub fn collect_leave_business_dates(
    conn: &rusqlite::Connection,
    user_id: i64,
    range_start: NaiveDate,
    range_end: NaiveDate,
    statuses: &[&str],
) -> HashSet<NaiveDate> {
    collect_leave_business_dates_filtered(conn, user_id, range_start, range_end, statuses, None)
}

/// Like collect_leave_business_dates but optionally filter by leave_type slugs.
pub fn collect_leave_business_dates_filtered(
    conn: &rusqlite::Connection,
    user_id: i64,
    range_start: NaiveDate,
    range_end: NaiveDate,
    statuses: &[&str],
    leave_type_slugs: Option<&[String]>,
) -> HashSet<NaiveDate> {
    if statuses.is_empty() || range_end < range_start {
        return HashSet::new();
    }

    let status_ph: String = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let slug_clause = if let Some(slugs) = leave_type_slugs {
        if slugs.is_empty() {
            return HashSet::new();
        }
        let sp: String = slugs.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        format!(" AND leave_type IN ({sp})")
    } else {
        String::new()
    };
    let sql = format!(
        "SELECT start_date, end_date FROM leave_requests
         WHERE user_id=? AND deleted_at IS NULL AND status IN ({status_ph})
           AND start_date <= ? AND end_date >= ?{slug_clause}",
    );

    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(user_id),
        Box::new(range_end.format("%Y-%m-%d").to_string()),
        Box::new(range_start.format("%Y-%m-%d").to_string()),
    ];
    for s in statuses {
        params.push(Box::new(*s));
    }
    if let Some(slugs) = leave_type_slugs {
        for slug in slugs {
            params.push(Box::new(slug.clone()));
        }
    }

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return HashSet::new(),
    };

    let rows: Vec<(String, String)> = match stmt.query_map(
        rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
        |row| Ok((row.get(0)?, row.get(1)?)),
    ) {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => return HashSet::new(),
    };

    let mut dates = HashSet::new();
    for (start, end) in rows {
        let Some(ls) = NaiveDate::parse_from_str(&start, "%Y-%m-%d").ok() else {
            continue;
        };
        let Some(le) = NaiveDate::parse_from_str(&end, "%Y-%m-%d").ok() else {
            continue;
        };
        dates.extend(working_dates_between_for_user(
            conn,
            user_id,
            ls.max(range_start),
            le.min(range_end),
        ));
    }
    dates
}

/// Per-date max LOP weight from approved leave (paid=0, half_day=0.5, unpaid=1).
pub fn collect_approved_leave_lop_weights(
    conn: &rusqlite::Connection,
    user_id: i64,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> HashMap<NaiveDate, f64> {
    if range_end < range_start {
        return HashMap::new();
    }
    let sql = "SELECT start_date, end_date, leave_type FROM leave_requests
         WHERE user_id=? AND deleted_at IS NULL AND status='approved'
           AND start_date <= ? AND end_date >= ?";
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    let rows: Vec<(String, String, String)> = stmt
        .query_map(
            rusqlite::params![
                user_id,
                range_end.format("%Y-%m-%d").to_string(),
                range_start.format("%Y-%m-%d").to_string(),
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut weights: HashMap<NaiveDate, f64> = HashMap::new();
    for (start, end, leave_type) in rows {
        let factor = crate::leave_type_logic::lop_factor_for_slug(conn, &leave_type);
        if factor <= 0.0 {
            continue;
        }
        let Some(ls) = NaiveDate::parse_from_str(&start, "%Y-%m-%d").ok() else {
            continue;
        };
        let Some(le) = NaiveDate::parse_from_str(&end, "%Y-%m-%d").ok() else {
            continue;
        };
        for d in working_dates_between_for_user(conn, user_id, ls.max(range_start), le.min(range_end))
        {
            weights
                .entry(d)
                .and_modify(|w| *w = w.max(factor))
                .or_insert(factor);
        }
    }
    weights
}

/// Total LOP-equivalent days in month (supports fractional half-days).
pub fn total_lop_days_for_month(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> f64 {
    let (active_start, active_end) = user_active_range_in_month(conn, user_id, month, year);
    if active_end < active_start {
        return 0.0;
    }
    let present = collect_present_business_dates(conn, user_id, active_start, active_end);
    let holidays = collect_paid_holiday_dates(conn, user_id, month, year);
    let leave_lop = collect_approved_leave_lop_weights(conn, user_id, active_start, active_end);
    let all_approved =
        collect_leave_business_dates(conn, user_id, active_start, active_end, &["approved"]);

    let mut total = 0.0;
    let mut d = active_start;
    while d <= active_end {
        if is_working_day_for_user(conn, user_id, d) {
            if present.contains(&d) || holidays.contains(&d) {
                // no LOP
            } else if let Some(w) = leave_lop.get(&d) {
                total += w;
            } else if all_approved.contains(&d) {
                // paid approved leave — no LOP
            } else {
                total += 1.0;
            }
        }
        d += chrono::Duration::days(1);
    }
    total
}

/// Completed attendance days (clock-out required), clipped to active range.
pub fn collect_present_business_dates(
    conn: &rusqlite::Connection,
    user_id: i64,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> HashSet<NaiveDate> {
    if range_end < range_start {
        return HashSet::new();
    }
    let start_s = range_start.format("%Y-%m-%d").to_string();
    let end_s = range_end.format("%Y-%m-%d").to_string();

    let mut stmt = match conn.prepare(
        "SELECT DISTINCT date FROM attendance
         WHERE user_id=?1 AND date >= ?2 AND date <= ?3 AND deleted_at IS NULL
           AND clock_out IS NOT NULL",
    ) {
        Ok(s) => s,
        Err(_) => return HashSet::new(),
    };

    stmt.query_map(rusqlite::params![user_id, &start_s, &end_s], |row| {
        row.get::<_, String>(0)
    })
    .ok()
    .map(|iter| {
        iter.filter_map(|r| r.ok())
            .filter_map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
            .filter(|d| is_working_day_for_user(conn, user_id, *d))
            .collect()
    })
    .unwrap_or_default()
}

/// Approved leave business days in month (distinct dates — overlapping requests deduped).
pub fn employee_leave_business_days(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> i64 {
    let (active_start, active_end) = user_active_range_in_month(conn, user_id, month, year);
    if active_end < active_start {
        return 0;
    }
    collect_leave_business_dates(conn, user_id, active_start, active_end, &["approved"]).len() as i64
}

/// Used approved quota-counting leave business days in calendar year.
pub fn employee_leave_used_in_year(conn: &rusqlite::Connection, user_id: i64, year: i32) -> i64 {
    let year_start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let year_end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
    let slugs = crate::leave_type_logic::quota_slugs(conn);
    if slugs.is_empty() {
        return 0;
    }
    collect_leave_business_dates_filtered(
        conn,
        user_id,
        year_start,
        year_end,
        &["approved"],
        Some(&slugs),
    )
    .len() as i64
}

/// Pending quota-counting leave business days in calendar year (deduped).
pub fn employee_pending_leave_days_in_year(conn: &rusqlite::Connection, user_id: i64, year: i32) -> i64 {
    let year_start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let year_end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
    let slugs = crate::leave_type_logic::quota_slugs(conn);
    if slugs.is_empty() {
        return 0;
    }
    collect_leave_business_dates_filtered(
        conn,
        user_id,
        year_start,
        year_end,
        &["pending"],
        Some(&slugs),
    )
    .len() as i64
}

/// Whether adding a new quota-counting leave request would exceed annual quota.
pub fn would_exceed_annual_quota(
    conn: &rusqlite::Connection,
    user_id: i64,
    start_date: &str,
    end_date: &str,
    leave_type: &str,
) -> bool {
    if !crate::leave_type_logic::counts_toward_quota(conn, leave_type) {
        return false;
    }
    let Some(ls) = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").ok() else {
        return false;
    };
    let Some(le) = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").ok() else {
        return false;
    };
    let year = ls.year();
    let year_start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let year_end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
    let slugs = crate::leave_type_logic::quota_slugs(conn);
    if slugs.is_empty() {
        return false;
    }

    let mut dates = collect_leave_business_dates_filtered(
        conn,
        user_id,
        year_start,
        year_end,
        &["approved", "pending"],
        Some(&slugs),
    );
    dates.extend(working_dates_between_for_user(
        conn,
        user_id,
        ls.max(year_start),
        le.min(year_end),
    ));

    dates.len() as i64 > annual_leave_quota(conn)
}

/// Total distinct approved leave business days across all employees in a month.
pub fn approved_leave_business_days_in_month(
    conn: &rusqlite::Connection,
    month: i32,
    year: i32,
) -> i64 {
    let (month_start, month_end) = month_bounds(month, year);
    let end_str = month_end.format("%Y-%m-%d").to_string();
    let start_str = month_start.format("%Y-%m-%d").to_string();

    let mut stmt = match conn.prepare(
        "SELECT user_id, start_date, end_date FROM leave_requests
         WHERE status='approved' AND deleted_at IS NULL
           AND start_date <= ?1 AND end_date >= ?2",
    ) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let rows: Vec<(i64, String, String)> = stmt
        .query_map(rusqlite::params![&end_str, &start_str], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let mut dates = HashSet::new();
    for (user_id, start, end) in rows {
        let Some(ls) = NaiveDate::parse_from_str(&start, "%Y-%m-%d").ok() else {
            continue;
        };
        let Some(le) = NaiveDate::parse_from_str(&end, "%Y-%m-%d").ok() else {
            continue;
        };
        dates.extend(working_dates_between_for_user(
            conn,
            user_id,
            ls.max(month_start),
            le.min(month_end),
        ));
    }
    dates.len() as i64
}

pub fn annual_leave_quota(conn: &rusqlite::Connection) -> i64 {
    conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM app_settings WHERE key='annual_leave_quota'",
        [],
        |r| r.get(0),
    )
    .unwrap_or(12)
}

pub fn employee_present_business_days(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> i64 {
    let (active_start, active_end) = user_active_range_in_month(conn, user_id, month, year);
    if active_end < active_start {
        return 0;
    }
    collect_present_business_dates(conn, user_id, active_start, active_end).len() as i64
}

pub fn employee_absent_business_days(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
    _working_days: i64,
    _paid_holidays: i64,
) -> i64 {
    collect_absent_business_dates(conn, user_id, month, year).len() as i64
}

/// Distinct paid-holiday business dates for a user in the active month range.
pub fn collect_paid_holiday_dates(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> HashSet<NaiveDate> {
    let (active_start, active_end) = user_active_range_in_month(conn, user_id, month, year);
    if active_end < active_start {
        return HashSet::new();
    }
    let start_s = month_bounds(month, year).0.format("%Y-%m-%d").to_string();
    let end_s = month_bounds(month, year).1.format("%Y-%m-%d").to_string();

    let mut stmt = match conn.prepare(
        "SELECT date FROM holidays WHERE date >= ?1 AND date <= ?2",
    ) {
        Ok(s) => s,
        Err(_) => return HashSet::new(),
    };

    stmt.query_map(rusqlite::params![start_s, end_s], |row| row.get::<_, String>(0))
        .ok()
        .map(|iter| {
            iter.filter_map(|r| r.ok())
                .filter_map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .filter(|d| {
                    is_working_day_for_user(conn, user_id, *d)
                        && *d >= active_start
                        && *d <= active_end
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Business dates with full-day unexcused absence (excludes paid leave and partial LOP days).
pub fn collect_absent_business_dates(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
) -> HashSet<NaiveDate> {
    let (active_start, active_end) = user_active_range_in_month(conn, user_id, month, year);
    if active_end < active_start {
        return HashSet::new();
    }
    let present = collect_present_business_dates(conn, user_id, active_start, active_end);
    let leave_lop = collect_approved_leave_lop_weights(conn, user_id, active_start, active_end);
    let all_approved =
        collect_leave_business_dates(conn, user_id, active_start, active_end, &["approved"]);
    let holidays = collect_paid_holiday_dates(conn, user_id, month, year);

    let mut absent = HashSet::new();
    let mut d = active_start;
    while d <= active_end {
        if !is_working_day_for_user(conn, user_id, d) || present.contains(&d) || holidays.contains(&d) {
            d += chrono::Duration::days(1);
            continue;
        }
        if let Some(w) = leave_lop.get(&d) {
            if *w >= 1.0 {
                absent.insert(d);
            }
        } else if !all_approved.contains(&d) {
            absent.insert(d);
        }
        d += chrono::Duration::days(1);
    }
    absent
}

/// Per-component LOP line (from salary_components earnings).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct LopLine {
    pub component_id: i64,
    pub name: String,
    pub amount: f64,
}

/// Per-component LOP breakdown (matches Excel red section).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct LopBreakdown {
    pub days: f64,
    pub lines: Vec<LopLine>,
    pub basic: f64,
    pub hra: f64,
    pub conveyance: f64,
    pub special: f64,
    pub total: f64,
    pub net_after_lop: f64,
}

pub fn component_lop_breakdown(
    salary: &crate::salary_logic::PayrollSalaryBreakdown,
    lop_days: f64,
    month: i32,
    year: i32,
) -> LopBreakdown {
    let divisor = calendar_days_in_month(month, year) as f64;
    if divisor <= 0.0 || lop_days <= 0.0 {
        return LopBreakdown {
            net_after_lop: salary.gross,
            ..Default::default()
        };
    }

    let mut lines = Vec::new();
    let mut basic = 0.0;
    let mut hra = 0.0;
    let mut conveyance = 0.0;
    let mut special = 0.0;

    for comp in &salary.components {
        let comp_type = comp.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if comp_type != "earning" {
            continue;
        }
        if comp
            .get("is_reimbursement")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        let amount = comp.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if amount <= 0.0 {
            continue;
        }
        let name = comp
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Earning")
            .to_string();
        let slug = comp
            .get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let comp_id = comp
            .get("component_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let lop_amt = crate::salary_split::round2(amount * lop_days / divisor);
        if lop_amt <= 0.0 {
            continue;
        }
        lines.push(LopLine {
            component_id: comp_id,
            name: format!("LOP — {name}"),
            amount: lop_amt,
        });
        match crate::salary_logic::bucket_component(slug, &name) {
            "basic" => basic = crate::salary_split::round2(basic + lop_amt),
            "hra" => hra = crate::salary_split::round2(hra + lop_amt),
            "transport" => conveyance = crate::salary_split::round2(conveyance + lop_amt),
            _ => special = crate::salary_split::round2(special + lop_amt),
        }
    }

    let total = crate::salary_split::round2(basic + hra + conveyance + special);
    LopBreakdown {
        days: lop_days,
        lines,
        basic,
        hra,
        conveyance,
        special,
        total,
        net_after_lop: crate::salary_split::round2((salary.gross - total).max(0.0)),
    }
}

/// LOP with per-component split; uses calendar days in month as divisor.
pub fn lop_amount_for_user_month(
    conn: &rusqlite::Connection,
    user_id: i64,
    month: i32,
    year: i32,
    _working_days: i64,
) -> (f64, LopBreakdown) {
    let lop_days = total_lop_days_for_month(conn, user_id, month, year);
    if lop_days <= 0.0 {
        return (0.0, LopBreakdown::default());
    }
    let cal_days = calendar_days_in_month(month, year);
    let month_end = format!("{}-{:02}-{}", year, month, cal_days);
    let Some(salary) = crate::salary_logic::load_user_salary(conn, user_id, &month_end) else {
        return (0.0, LopBreakdown::default());
    };
    let breakdown = component_lop_breakdown(&salary, lop_days, month, year);
    (breakdown.total, breakdown)
}

/// Sum of per-employee paid holidays across active staff for a month.
pub fn total_paid_holidays_for_month(
    conn: &rusqlite::Connection,
    month: i32,
    year: i32,
) -> i64 {
    let mut stmt = match conn.prepare(
        "SELECT id FROM users WHERE deleted_at IS NULL AND is_super_admin=0",
    ) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let user_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    user_ids
        .iter()
        .map(|uid| paid_holidays_for_user(conn, *uid, month, year))
        .sum()
}

/// Approved leave business days in month grouped by leave type (deduped per date per type).
pub fn approved_leave_days_by_type_in_month(
    conn: &rusqlite::Connection,
    month: i32,
    year: i32,
) -> HashMap<String, i64> {
    let (month_start, month_end) = month_bounds(month, year);
    let end_str = month_end.format("%Y-%m-%d").to_string();
    let start_str = month_start.format("%Y-%m-%d").to_string();

    let mut stmt = match conn.prepare(
        "SELECT user_id, leave_type, start_date, end_date FROM leave_requests
         WHERE status='approved' AND deleted_at IS NULL
           AND start_date <= ?1 AND end_date >= ?2",
    ) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };

    let rows: Vec<(i64, String, String, String)> = stmt
        .query_map(rusqlite::params![&end_str, &start_str], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let mut by_type: HashMap<String, HashSet<NaiveDate>> = HashMap::new();
    for (user_id, leave_type, start, end) in rows {
        let Some(ls) = NaiveDate::parse_from_str(&start, "%Y-%m-%d").ok() else {
            continue;
        };
        let Some(le) = NaiveDate::parse_from_str(&end, "%Y-%m-%d").ok() else {
            continue;
        };
        let dates = working_dates_between_for_user(
            conn,
            user_id,
            ls.max(month_start),
            le.min(month_end),
        );
        by_type.entry(leave_type).or_default().extend(dates);
    }
    by_type.into_iter().map(|(k, v)| (k, v.len() as i64)).collect()
}

/// Parse per-employee adjustments from preview request body.
pub fn parse_employee_adjustments(
    adjustments: &Option<serde_json::Value>,
) -> HashMap<i64, Vec<serde_json::Value>> {
    let mut map = HashMap::new();
    let Some(obj) = adjustments.as_ref().and_then(|v| v.as_object()) else {
        return map;
    };
    for (key, val) in obj {
        let Ok(uid) = key.parse::<i64>() else { continue };
        if let Some(arr) = val.as_array() {
            map.insert(uid, arr.clone());
        }
    }
    map
}

/// Apply a list of flat adjustments; returns (net, total_deductions, json).
pub fn apply_adjustment_list(
    gross: f64,
    mut net: f64,
    mut total_deductions: f64,
    adjs: &[serde_json::Value],
) -> (f64, f64, String) {
    let mut applied = Vec::new();
    for adj in adjs {
        let adj_type = adj.get("type").and_then(|v| v.as_str()).unwrap_or("addition");
        let value_type = adj
            .get("value_type")
            .and_then(|v| v.as_str())
            .unwrap_or("flat");
        let value = adj.get("value").and_then(|v| v.as_f64()).or_else(|| {
            adj.get("amount").and_then(|v| v.as_f64())
        }).unwrap_or(0.0);
        if value == 0.0 {
            continue;
        }
        let amount = if value_type == "percentage" {
            gross * value / 100.0
        } else {
            value
        };
        if adj_type == "addition" {
            net += amount;
        } else {
            net = (net - amount).max(0.0);
            total_deductions += amount;
        }
        applied.push(serde_json::json!({
            "type": adj_type,
            "label": adj.get("label").and_then(|v| v.as_str()).unwrap_or(""),
            "value_type": value_type,
            "value": value,
            "amount": amount,
        }));
    }
    let json = serde_json::to_string(&applied).unwrap_or_else(|_| "[]".to_string());
    (net, total_deductions, json)
}
