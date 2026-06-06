//! Leave type configuration — paid / unpaid / half-day LOP rules.

use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LeaveTypeConfig {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub payment_type: String,
    pub counts_toward_quota: bool,
    pub is_active: bool,
}

/// LOP weight for one business day: 0 = paid, 0.5 = half-day, 1.0 = unpaid.
pub fn lop_factor(payment_type: &str) -> f64 {
    match payment_type {
        "unpaid" => 1.0,
        "half_day" => 0.5,
        _ => 0.0,
    }
}

pub fn payment_type_label(payment_type: &str) -> &'static str {
    match payment_type {
        "unpaid" => "Unpaid (LOP)",
        "half_day" => "Half-day (50% LOP)",
        _ => "Paid",
    }
}

pub fn load_all(conn: &Connection) -> Vec<LeaveTypeConfig> {
    let mut stmt = match conn.prepare(
        "SELECT id, slug, name, payment_type, counts_toward_quota, is_active
         FROM leave_types ORDER BY name",
    ) {
        Ok(s) => s,
        Err(_) => return default_types(),
    };
    let rows = stmt
        .query_map([], |row| {
            Ok(LeaveTypeConfig {
                id: row.get(0)?,
                slug: row.get(1)?,
                name: row.get(2)?,
                payment_type: row.get(3)?,
                counts_toward_quota: row.get::<_, i64>(4).unwrap_or(0) != 0,
                is_active: row.get::<_, i64>(5).unwrap_or(1) != 0,
            })
        })
        .ok();
    match rows {
        Some(iter) => {
            let list: Vec<_> = iter.filter_map(|r| r.ok()).collect();
            if list.is_empty() {
                default_types()
            } else {
                list
            }
        }
        None => default_types(),
    }
}

pub fn load_active(conn: &Connection) -> Vec<LeaveTypeConfig> {
    load_all(conn)
        .into_iter()
        .filter(|t| t.is_active)
        .collect()
}

pub fn load_map(conn: &Connection) -> HashMap<String, LeaveTypeConfig> {
    load_all(conn)
        .into_iter()
        .map(|t| (t.slug.clone(), t))
        .collect()
}

pub fn config_for_slug(conn: &Connection, slug: &str) -> Option<LeaveTypeConfig> {
    load_map(conn).into_iter().find(|(s, _)| s == slug).map(|(_, c)| c)
}

pub fn lop_factor_for_slug(conn: &Connection, slug: &str) -> f64 {
    config_for_slug(conn, slug)
        .map(|c| lop_factor(&c.payment_type))
        .unwrap_or_else(|| {
            if slug == "unpaid" {
                1.0
            } else {
                0.0
            }
        })
}

pub fn counts_toward_quota(conn: &Connection, slug: &str) -> bool {
    config_for_slug(conn, slug)
        .map(|c| c.counts_toward_quota)
        .unwrap_or(slug == "annual")
}

pub fn quota_slugs(conn: &Connection) -> Vec<String> {
    load_all(conn)
        .into_iter()
        .filter(|t| t.counts_toward_quota && t.is_active)
        .map(|t| t.slug)
        .collect()
}

fn default_types() -> Vec<LeaveTypeConfig> {
    vec![
        LeaveTypeConfig {
            id: 0,
            slug: "sick".into(),
            name: "Sick Leave".into(),
            payment_type: "paid".into(),
            counts_toward_quota: false,
            is_active: true,
        },
        LeaveTypeConfig {
            id: 0,
            slug: "annual".into(),
            name: "Annual Leave".into(),
            payment_type: "paid".into(),
            counts_toward_quota: true,
            is_active: true,
        },
        LeaveTypeConfig {
            id: 0,
            slug: "personal".into(),
            name: "Personal Leave".into(),
            payment_type: "paid".into(),
            counts_toward_quota: false,
            is_active: true,
        },
        LeaveTypeConfig {
            id: 0,
            slug: "unpaid".into(),
            name: "Unpaid Leave".into(),
            payment_type: "unpaid".into(),
            counts_toward_quota: false,
            is_active: true,
        },
        LeaveTypeConfig {
            id: 0,
            slug: "emergency".into(),
            name: "Emergency Leave".into(),
            payment_type: "paid".into(),
            counts_toward_quota: false,
            is_active: true,
        },
    ]
}
