//! Shared attendance session resolution.

/// Returns (attendance_id, session_date, clock_in) for the user's open session.
/// Prefers same-day; falls back to the most recent open session (overnight shifts).
pub fn find_open_attendance_session(
    conn: &rusqlite::Connection,
    user_id: i64,
    punch_date: &str,
) -> Option<(i64, String, String)> {
    if let Ok(row) = conn.query_row(
        "SELECT id, date, clock_in FROM attendance
         WHERE user_id=?1 AND date=?2 AND clock_out IS NULL AND deleted_at IS NULL
         ORDER BY id DESC LIMIT 1",
        rusqlite::params![user_id, punch_date],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ) {
        return Some(row);
    }
    conn.query_row(
        "SELECT id, date, clock_in FROM attendance
         WHERE user_id=?1 AND clock_out IS NULL AND deleted_at IS NULL
           AND date < ?2
         ORDER BY date DESC, id DESC LIMIT 1",
        rusqlite::params![user_id, punch_date],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .ok()
}

/// Combine date+time; rolls clock-out to next day when out < in (overnight).
pub fn combine_clock_out_datetime(date: &str, clock_in: &str, clock_out: &str) -> String {
    let in_part = if clock_in.len() >= 8 {
        &clock_in[..8]
    } else {
        clock_in
    };
    let out_part = if clock_out.len() >= 8 {
        &clock_out[..8]
    } else {
        clock_out
    };
    let out_date = if out_part < in_part {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            (d + chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string()
        } else {
            date.to_string()
        }
    } else {
        date.to_string()
    };
    format!("{}T{}", out_date, out_part)
}

/// Close any open attendance session (including prior-day overnight) before clock-in.
pub fn close_open_session_before_clock_in(
    conn: &rusqlite::Connection,
    user_id: i64,
    punch_date: &str,
    clock_out_time: &str,
    updated_at: &str,
    today_shift: &crate::shift_logic::ShiftConfig,
) {
    use crate::shift_logic::{close_open_sessions, resolve_shift_for_user};

    for _ in 0..8 {
        let Some((_, session_date, _)) = find_open_attendance_session(conn, user_id, punch_date) else {
            break;
        };
        let session_shift = resolve_shift_for_user(conn, user_id, &session_date);
        close_open_sessions(
            conn,
            user_id,
            &session_date,
            clock_out_time,
            updated_at,
            &session_shift,
        );
    }
    close_open_sessions(conn, user_id, punch_date, clock_out_time, updated_at, today_shift);
}

pub fn combine_datetime(date: &str, time: &str) -> String {
    let time_part = if time.len() >= 8 { &time[..8] } else { time };
    format!("{}T{}", date, time_part)
}
