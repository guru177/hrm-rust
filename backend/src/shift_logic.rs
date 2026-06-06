use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Weekday};
use rusqlite::Connection;

pub const DEFAULT_WORK_START: &str = "09:00:00";
pub const DEFAULT_WORK_END: &str = "18:00:00";
pub const GENERAL_SHIFT_NAME: &str = "General";
/// Mon–Fri (bits: Mon=1, Tue=2, Wed=4, Thu=8, Fri=16, Sat=32, Sun=64)
pub const DEFAULT_WORKING_DAYS_MASK: u8 = 31;

const WEEKDAY_DEFS: [(&str, u8, &str); 7] = [
    ("mon", 1, "Mon"),
    ("tue", 2, "Tue"),
    ("wed", 4, "Wed"),
    ("thu", 8, "Thu"),
    ("fri", 16, "Fri"),
    ("sat", 32, "Sat"),
    ("sun", 64, "Sun"),
];

pub fn weekday_bit(d: NaiveDate) -> u8 {
    match d.weekday() {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 4,
        Weekday::Thu => 8,
        Weekday::Fri => 16,
        Weekday::Sat => 32,
        Weekday::Sun => 64,
    }
}

pub fn is_working_day(mask: u8, d: NaiveDate) -> bool {
    let m = if mask == 0 {
        DEFAULT_WORKING_DAYS_MASK
    } else {
        mask
    };
    weekday_bit(d) & m != 0
}

pub fn normalize_working_days_mask(mask: i64) -> u8 {
    let m = mask as u8;
    if m == 0 {
        DEFAULT_WORKING_DAYS_MASK
    } else {
        m
    }
}

pub fn mask_from_weekday_keys(keys: &[impl AsRef<str>]) -> u8 {
    let mut mask = 0u8;
    for key in keys {
        let k = key.as_ref().trim().to_lowercase();
        for (slug, bit, _) in WEEKDAY_DEFS {
            if k == slug {
                mask |= bit;
            }
        }
    }
    if mask == 0 {
        DEFAULT_WORKING_DAYS_MASK
    } else {
        mask
    }
}

pub fn mask_to_weekday_keys(mask: u8) -> Vec<String> {
    let m = normalize_working_days_mask(mask as i64);
    WEEKDAY_DEFS
        .iter()
        .filter(|(_, bit, _)| m & bit != 0)
        .map(|(slug, _, _)| (*slug).to_string())
        .collect()
}

pub fn format_working_days_label(mask: u8) -> String {
    let m = normalize_working_days_mask(mask as i64);
    let labels: Vec<&str> = WEEKDAY_DEFS
        .iter()
        .filter(|(_, bit, _)| m & bit != 0)
        .map(|(_, _, label)| *label)
        .collect();
    if labels.is_empty() {
        "—".to_string()
    } else {
        labels.join(", ")
    }
}

pub fn working_days_mask_for_user(conn: &Connection, user_id: i64, as_of: &str) -> u8 {
    resolve_shift_for_user(conn, user_id, as_of).working_days_mask
}

#[derive(Debug, Clone)]
pub struct ShiftConfig {
    pub template_id: Option<i64>,
    pub template_name: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub grace_in_minutes: i64,
    pub grace_out_minutes: i64,
    pub working_days_mask: u8,
    /// True when daily roster marks this date as off.
    pub is_day_off: bool,
    /// `daily` | `assignment` | `default`
    pub schedule_source: String,
}

impl ShiftConfig {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "template_id": self.template_id,
            "template_name": self.template_name,
            "start_time": self.start_time,
            "end_time": self.end_time,
            "grace_in_minutes": self.grace_in_minutes,
            "grace_out_minutes": self.grace_out_minutes,
            "working_days_mask": self.working_days_mask,
            "working_days": mask_to_weekday_keys(self.working_days_mask),
            "working_days_label": format_working_days_label(self.working_days_mask),
            "is_day_off": self.is_day_off,
            "schedule_source": self.schedule_source,
        })
    }
}

fn shift_config_from_template_row(row: &rusqlite::Row<'_>, source: &str, is_day_off: bool) -> rusqlite::Result<ShiftConfig> {
    Ok(ShiftConfig {
        template_id: row.get::<_, i64>(0).ok(),
        template_name: row.get::<_, String>(1).ok(),
        start_time: row
            .get::<_, String>(2)
            .unwrap_or_else(|_| DEFAULT_WORK_START.to_string()),
        end_time: row
            .get::<_, String>(3)
            .unwrap_or_else(|_| DEFAULT_WORK_END.to_string()),
        grace_in_minutes: row.get::<_, i64>(4).unwrap_or(0).max(0),
        grace_out_minutes: row.get::<_, i64>(5).unwrap_or(0).max(0),
        working_days_mask: normalize_working_days_mask(
            row.get::<_, i64>(6).unwrap_or(DEFAULT_WORKING_DAYS_MASK as i64),
        ),
        is_day_off,
        schedule_source: source.to_string(),
    })
}

const SHIFT_TEMPLATE_SELECT: &str = "st.id, st.name, st.start_time, st.end_time, st.grace_in_minutes, st.grace_out_minutes,
                COALESCE(st.working_days_mask, 31) AS working_days_mask";

fn query_shift_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    date: &str,
) -> Option<ShiftConfig> {
    conn.query_row(
        &format!(
            "SELECT {SHIFT_TEMPLATE_SELECT}
         FROM user_shift_assignments usa
         JOIN shift_templates st ON st.id = usa.shift_template_id
         WHERE usa.user_id = ?1
           AND usa.effective_from <= ?2
           AND (usa.effective_to IS NULL OR usa.effective_to >= ?2)
         ORDER BY usa.effective_from DESC, usa.id DESC
         LIMIT 1"
        ),
        rusqlite::params![user_id, date],
        |row| shift_config_from_template_row(row, "assignment", false),
    )
    .ok()
}

/// Daily roster override for a single date (if any).
pub fn query_daily_roster(
    conn: &Connection,
    user_id: i64,
    date: &str,
) -> Option<(bool, Option<ShiftConfig>)> {
    let row: (i64, Option<i64>) = conn
        .query_row(
            "SELECT COALESCE(is_day_off, 0), shift_template_id
             FROM shift_daily_roster
             WHERE user_id = ?1 AND roster_date = ?2",
            rusqlite::params![user_id, date],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1).ok().flatten())),
        )
        .ok()?;

    if row.0 != 0 {
        return Some((true, None));
    }
    let Some(shift_id) = row.1 else {
        return Some((false, None));
    };
    let shift = conn
        .query_row(
            &format!(
                "SELECT {SHIFT_TEMPLATE_SELECT}
             FROM shift_templates st
             WHERE st.id = ?1"
            ),
            [shift_id],
            |r| shift_config_from_template_row(r, "daily", false),
        )
        .ok();
    Some((false, shift))
}

/// Whether payroll should treat this calendar day as a working day for the user.
pub fn user_is_scheduled_working_day(conn: &Connection, user_id: i64, date: &str, d: NaiveDate) -> bool {
    if let Some((is_off, daily_shift)) = query_daily_roster(conn, user_id, date) {
        if is_off {
            return false;
        }
        if daily_shift.is_some() {
            return true;
        }
    }
    let mask = resolve_shift_for_user(conn, user_id, date).working_days_mask;
    is_working_day(mask, d)
}

pub fn upsert_daily_roster(
    conn: &Connection,
    user_id: i64,
    roster_date: &str,
    shift_template_id: Option<i64>,
    is_day_off: bool,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if !is_day_off && shift_template_id.is_none() {
        return conn.execute(
            "DELETE FROM shift_daily_roster WHERE user_id=?1 AND roster_date=?2",
            rusqlite::params![user_id, roster_date],
        )
        .map(|_| ());
    }
    conn.execute(
        "INSERT INTO shift_daily_roster (user_id, roster_date, shift_template_id, is_day_off, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(user_id, roster_date) DO UPDATE SET
           shift_template_id=excluded.shift_template_id,
           is_day_off=excluded.is_day_off,
           updated_at=excluded.updated_at",
        rusqlite::params![
            user_id,
            roster_date,
            shift_template_id,
            if is_day_off { 1 } else { 0 },
            &now,
        ],
    )
    .map(|_| ())
}

pub fn user_has_active_assignment(
    conn: &rusqlite::Connection,
    user_id: i64,
    date: &str,
) -> bool {
    conn.query_row(
        "SELECT 1 FROM user_shift_assignments usa
         WHERE usa.user_id = ?1
           AND usa.effective_from <= ?2
           AND (usa.effective_to IS NULL OR usa.effective_to >= ?2)
         LIMIT 1",
        rusqlite::params![user_id, date],
        |_| Ok(()),
    )
    .is_ok()
}

/// Ensures the default shift template exists (auto-assigned to unassigned employees).
pub fn ensure_general_shift_template(conn: &rusqlite::Connection) -> i64 {
    if let Ok(id) = conn.query_row(
        "SELECT id FROM shift_templates WHERE is_default = 1 ORDER BY id ASC LIMIT 1",
        [],
        |row| row.get::<_, i64>(0),
    ) {
        return id;
    }

    if let Ok(id) = conn.query_row(
        "SELECT id FROM shift_templates WHERE LOWER(name) = LOWER(?1) ORDER BY id ASC LIMIT 1",
        [GENERAL_SHIFT_NAME],
        |row| row.get::<_, i64>(0),
    ) {
        let _ = conn.execute("UPDATE shift_templates SET is_default = 0", []);
        let _ = conn.execute(
            "UPDATE shift_templates SET is_default = 1 WHERE id = ?1",
            [id],
        );
        return id;
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = conn.execute("UPDATE shift_templates SET is_default = 0", []);
    let _ = conn.execute(
        "INSERT INTO shift_templates (name, start_time, end_time, grace_in_minutes, grace_out_minutes, is_active, is_default, working_days_mask, created_at, updated_at)
         VALUES (?1, ?2, ?3, 0, 0, 1, 1, ?4, ?5, ?5)",
        rusqlite::params![
            GENERAL_SHIFT_NAME,
            DEFAULT_WORK_START,
            DEFAULT_WORK_END,
            DEFAULT_WORKING_DAYS_MASK as i64,
            &now,
        ],
    );
    conn.last_insert_rowid()
}

/// Assigns the General shift when the employee has no active assignment on `effective_from`.
pub fn assign_general_shift_to_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    effective_from: &str,
) -> Result<(), rusqlite::Error> {
    if user_has_active_assignment(conn, user_id, effective_from) {
        return Ok(());
    }

    let shift_id = ensure_general_shift_template(conn);
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO user_shift_assignments (user_id, shift_template_id, effective_from, effective_to, created_at, updated_at)
         VALUES (?1, ?2, ?3, NULL, ?4, ?4)",
        rusqlite::params![user_id, shift_id, effective_from, &now],
    )?;
    Ok(())
}

/// Merge duplicate "general" templates into the single default shift and remove extras.
pub fn consolidate_duplicate_general_shifts(conn: &rusqlite::Connection) {
    let default_id = ensure_general_shift_template(conn);

    let dup_ids: Vec<i64> = match conn.prepare(
        "SELECT id FROM shift_templates WHERE LOWER(name) = 'general' AND id != ?1",
    ) {
        Ok(mut stmt) => stmt
            .query_map([default_id], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect(),
        Err(_) => return,
    };

    for dup_id in dup_ids {
        let _ = conn.execute(
            "DELETE FROM user_shift_assignments
             WHERE shift_template_id = ?1
               AND user_id IN (
                   SELECT user_id FROM user_shift_assignments WHERE shift_template_id = ?2
               )",
            rusqlite::params![dup_id, default_id],
        );
        let _ = conn.execute(
            "UPDATE user_shift_assignments SET shift_template_id = ?1 WHERE shift_template_id = ?2",
            rusqlite::params![default_id, dup_id],
        );
        let _ = conn.execute("DELETE FROM shift_templates WHERE id = ?1", [dup_id]);
    }
}

/// Backfill General shift for all active employees without a current assignment.
pub fn backfill_general_shift_assignments(conn: &rusqlite::Connection) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let _ = ensure_general_shift_template(conn);

    let mut stmt = match conn.prepare(
        "SELECT u.id, COALESCE(NULLIF(u.date_of_joining, ''), substr(u.created_at, 1, 10), ?1)
         FROM users u
         WHERE u.deleted_at IS NULL
           AND NOT EXISTS (
               SELECT 1 FROM user_shift_assignments usa
               WHERE usa.user_id = u.id
                 AND usa.effective_from <= ?1
                 AND (usa.effective_to IS NULL OR usa.effective_to >= ?1)
           )",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rows: Vec<(i64, String)> = stmt
        .query_map([&today], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    for (user_id, effective_from) in rows {
        let _ = assign_general_shift_to_user(conn, user_id, &effective_from);
    }
}

pub fn resolve_shift_for_user(
    conn: &rusqlite::Connection,
    user_id: i64,
    date: &str,
) -> ShiftConfig {
    if let Some((is_off, daily)) = query_daily_roster(conn, user_id, date) {
        if let Some(shift) = daily {
            return shift;
        }
        if is_off {
            let mut base = query_shift_for_user(conn, user_id, date)
                .unwrap_or_else(|| default_shift_config());
            base.is_day_off = true;
            base.schedule_source = "daily".into();
            return base;
        }
    }

    if let Some(shift) = query_shift_for_user(conn, user_id, date) {
        return shift;
    }

    let _ = assign_general_shift_to_user(conn, user_id, date);

    query_shift_for_user(conn, user_id, date).unwrap_or_else(default_shift_config)
}

fn default_shift_config() -> ShiftConfig {
    ShiftConfig {
        template_id: None,
        template_name: Some(GENERAL_SHIFT_NAME.to_string()),
        start_time: DEFAULT_WORK_START.to_string(),
        end_time: DEFAULT_WORK_END.to_string(),
        grace_in_minutes: 0,
        grace_out_minutes: 0,
        working_days_mask: DEFAULT_WORKING_DAYS_MASK,
        is_day_off: false,
        schedule_source: "default".into(),
    }
}

pub fn parse_time(value: &str) -> Option<NaiveTime> {
    let trimmed = if value.len() >= 8 {
        &value[..8]
    } else {
        value
    };
    NaiveTime::parse_from_str(trimmed, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(trimmed, "%H:%M"))
        .ok()
}

fn add_minutes_wrap(base: NaiveTime, minutes: i64) -> NaiveTime {
    let secs = minutes * 60;
    if secs >= 0 {
        base.overflowing_add_signed(chrono::Duration::seconds(secs)).0
    } else {
        base.overflowing_sub_signed(chrono::Duration::seconds(secs.abs())).0
    }
}

/// Duration between clock-in and clock-out; supports overnight (out < in).
pub fn calc_duration_minutes(clock_in: &str, clock_out: &str) -> i64 {
    match (parse_time(clock_in), parse_time(clock_out)) {
        (Some(a), Some(b)) => {
            let diff = b.signed_duration_since(a);
            if diff.num_minutes() < 0 {
                diff.num_minutes() + 24 * 60
            } else {
                diff.num_minutes()
            }
        }
        _ => 0,
    }
}

fn minutes_of(t: chrono::NaiveTime) -> i64 {
    t.num_seconds_from_midnight() as i64 / 60
}

/// Late arrival check; supports overnight shifts (end < start).
pub fn is_late_arrival(
    clock_in: &str,
    work_start: &str,
    work_end: &str,
    grace_in_minutes: i64,
) -> bool {
    match (parse_time(clock_in), parse_time(work_start), parse_time(work_end)) {
        (Some(a), Some(b), Some(e)) => {
            let overnight = minutes_of(e) < minutes_of(b);
            let mut punch = minutes_of(a);
            let start = minutes_of(b);
            let end = minutes_of(e);
            if overnight && punch <= end {
                punch += 24 * 60;
            }
            punch > start + grace_in_minutes.max(0)
        }
        (Some(a), Some(b), None) => a > add_minutes_wrap(b, grace_in_minutes.max(0)),
        _ => false,
    }
}

/// Early departure check; supports overnight shifts (end < start).
pub fn is_early_departure(
    clock_out: &str,
    work_start: &str,
    work_end: &str,
    grace_out_minutes: i64,
) -> bool {
    match (parse_time(clock_out), parse_time(work_start), parse_time(work_end)) {
        (Some(a), Some(b), Some(e)) => {
            let overnight = minutes_of(e) < minutes_of(b);
            let out = minutes_of(a);
            let start = minutes_of(b);
            let end = minutes_of(e);
            let grace = grace_out_minutes.max(0);
            if overnight {
                if out >= start {
                    let out_adj = out;
                    let end_adj = end + 24 * 60;
                    return out_adj < end_adj - grace;
                }
                return out < end - grace;
            }
            out < end - grace
        }
        _ => false,
    }
}

/// Close open sessions before a new clock-in (manual or biometric).
pub fn close_open_sessions(
    conn: &rusqlite::Connection,
    user_id: i64,
    date: &str,
    clock_out_time: &str,
    updated_at: &str,
    shift: &ShiftConfig,
) {
    let mut stmt = match conn.prepare(
        "SELECT id, clock_in FROM attendance WHERE user_id=?1 AND date=?2 AND clock_out IS NULL AND deleted_at IS NULL",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };
    let rows: Vec<(i64, String)> = stmt
        .query_map(rusqlite::params![user_id, date], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    for (att_id, clock_in) in rows {
        let duration = calc_duration_minutes(&clock_in, clock_out_time);
        let early_exit = is_early_departure(
            clock_out_time,
            &shift.start_time,
            &shift.end_time,
            shift.grace_out_minutes,
        );
        let _ = conn.execute(
            "UPDATE attendance SET clock_out=?1, duration_minutes=?2, is_early_exit=?3, updated_at=?4 WHERE id=?5",
            rusqlite::params![
                clock_out_time,
                duration,
                if early_exit { 1 } else { 0 },
                updated_at,
                att_id,
            ],
        );
    }
}
