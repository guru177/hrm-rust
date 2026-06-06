use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web::error::ErrorUnauthorized;
use actix_ws::Message;
use futures_util::StreamExt as _;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use std::sync::Arc;

use crate::biometric_events::BiometricEvents;
use crate::db::DbPool;
use crate::middleware::auth::get_claims_from_request;
use crate::models::user::JwtClaims;
use crate::models::{ApiError, ApiResponse};
use crate::models::biometric::{BiometricDevice, BiometricPunch, BiometricUserMap, IClockQuery, UserMapRequest};

/// Update device heartbeat and push a live event to connected admin browsers.
fn record_device_touch(
    conn: &rusqlite::Connection,
    events: &BiometricEvents,
    sn: &str,
    ip: &str,
    event_kind: &str,
) {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = conn.execute(
        "INSERT INTO biometric_devices (serial_number, ip_address, last_heartbeat, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1, ?3, ?3)
         ON CONFLICT(serial_number) DO UPDATE SET ip_address=?2, last_heartbeat=?3, is_active=1, updated_at=?3",
        rusqlite::params![sn, ip, &now],
    );
    events.emit(
        event_kind,
        serde_json::json!({
            "serial_number": sn,
            "ip_address": ip,
            "last_heartbeat": now,
        }),
    );
}

// ═══════════════════════════════════════════════════════════════════
//  iClock / ADMS Protocol Endpoints (No Auth — Device-to-Server)
// ═══════════════════════════════════════════════════════════════════

/// GET /iclock/cdata — Device handshake / registration
/// The biometric device calls this on boot and periodically (heartbeat).
pub async fn iclock_handshake(
    pool: web::Data<DbPool>,
    events: web::Data<BiometricEvents>,
    query: web::Query<IClockQuery>,
    req: HttpRequest,
) -> HttpResponse {
    let sn = match &query.sn {
        Some(s) => s.clone(),
        None => return HttpResponse::BadRequest().body("ERR: Missing SN"),
    };

    let peer_ip = req.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    log::info!("🔗 [BIOMETRIC] Handshake from device SN={} IP={}", sn, peer_ip);

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("ERR: DB"),
    };

    record_device_touch(&conn, &events, &sn, &peer_ip, "device_online");

    // Return iClock-compatible response telling the device we accept it
    let response = format!(
        "GET OPTION FROM: {}\r\nStamp=9999\r\nOpStamp=9999\r\nPhotoStamp=9999\r\nErrorDelay=60\r\nDelay=10\r\nTransTimes=00:00;14:05\r\nTransInterval=1\r\nTransFlag=TransData AttLog\tOpLog\r\nRealtime=1\r\nEncrypt=0\r\n",
        sn
    );

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(response)
}

/// POST /iclock/cdata — Receive attendance logs (ATTLOG) and operation logs (OPERLOG)
/// This is the core endpoint where the device pushes punch data.
pub async fn iclock_receive(
    pool: web::Data<DbPool>,
    events: web::Data<BiometricEvents>,
    query: web::Query<IClockQuery>,
    body: String,
) -> HttpResponse {
    let sn = match &query.sn {
        Some(s) => s.clone(),
        None => return HttpResponse::BadRequest().body("ERR: Missing SN"),
    };

    let table = query.table.as_deref().unwrap_or("ATTLOG");

    log::info!("📥 [BIOMETRIC] Received data from SN={} table={} body_len={}", sn, table, body.len());

    if table == "ATTLOG" || table == "attlog" {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return HttpResponse::InternalServerError().body("ERR: DB"),
        };

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut processed = 0;

        // Parse tab-separated attendance records, one per line
        // Format: PIN\tTimestamp\tStatus\tVerify\tWorkCode\tReserved1\tReserved2
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 2 { continue; }

            let pin = fields[0].trim();
            let timestamp = fields.get(1).map(|s| s.trim()).unwrap_or("");
            let status: i64 = fields.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            let verify: i64 = fields.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(0);

            if pin.is_empty() || timestamp.is_empty() { continue; }

            log::info!("  📋 Punch: PIN={} Time={} Status={} Verify={}", pin, timestamp, status, verify);

            let user_id: Option<i64> = conn.query_row(
                "SELECT user_id FROM biometric_user_map WHERE device_serial=?1 AND device_pin=?2",
                rusqlite::params![&sn, pin],
                |row| row.get(0),
            ).ok();

            let Some(uid) = user_id else {
                log::info!("  ⏭️  Unmapped PIN={} on SN={} — punch ignored", pin, sn);
                continue;
            };

            // Check for duplicate (same device, pin, timestamp)
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) FROM biometric_punches WHERE device_serial=?1 AND device_pin=?2 AND punch_time=?3",
                rusqlite::params![&sn, pin, timestamp],
                |row| row.get::<_, i64>(0),
            ).unwrap_or(0) > 0;

            if exists {
                log::info!("  ⏭️  Duplicate punch skipped: PIN={} Time={}", pin, timestamp);
                continue;
            }

            if conn.execute(
                "INSERT INTO biometric_punches (device_serial, device_pin, punch_time, punch_type, verify_method, user_id, is_processed, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
                rusqlite::params![&sn, pin, timestamp, status, verify, uid, &now],
            ).is_ok() {
                let punch_id = conn.last_insert_rowid();
                process_punch_to_attendance(&conn, punch_id, uid, timestamp, status);
            }

            processed += 1;
        }

        // Update device heartbeat
        let _ = conn.execute(
            "UPDATE biometric_devices SET last_heartbeat=?1, updated_at=?1 WHERE serial_number=?2",
            rusqlite::params![&now, &sn],
        );

        log::info!("✅ [BIOMETRIC] Processed {} punches from SN={}", processed, sn);
        if processed > 0 {
            events.emit(
                "punches_received",
                serde_json::json!({
                    "serial_number": sn,
                    "count": processed,
                }),
            );
        }
    } else {
        log::info!("ℹ️  [BIOMETRIC] Ignoring table={} from SN={}", table, sn);
    }

    HttpResponse::Ok().content_type("text/plain").body("OK")
}

/// GET /iclock/getrequest — Device polls for pending commands
pub async fn iclock_getrequest(
    pool: web::Data<DbPool>,
    events: web::Data<BiometricEvents>,
    query: web::Query<IClockQuery>,
    req: HttpRequest,
) -> HttpResponse {
    let sn = match &query.sn {
        Some(s) => s.clone(),
        None => return HttpResponse::BadRequest().body("ERR: Missing SN"),
    };

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("ERR: DB"),
    };

    let peer_ip = req.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    record_device_touch(&conn, &events, &sn, &peer_ip, "device_heartbeat");

    // Check for pending commands
    let cmd: Option<(i64, String)> = conn.query_row(
        "SELECT id, command FROM biometric_commands WHERE device_serial=?1 AND status='pending' ORDER BY id LIMIT 1",
        rusqlite::params![&sn],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).ok();

    if let Some((cmd_id, command)) = cmd {
        // Mark as sent
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = conn.execute(
            "UPDATE biometric_commands SET status='sent', executed_at=?1 WHERE id=?2",
            rusqlite::params![&now, cmd_id],
        );
        log::info!("📤 [BIOMETRIC] Sending command to SN={}: {}", sn, command);
        HttpResponse::Ok().content_type("text/plain").body(format!("C:{}:{}", cmd_id, command))
    } else {
        HttpResponse::Ok().content_type("text/plain").body("OK")
    }
}

/// POST /iclock/devicecmd — Device reports command execution result
pub async fn iclock_devicecmd(pool: web::Data<DbPool>, query: web::Query<IClockQuery>, body: String) -> HttpResponse {
    let sn = query.sn.as_deref().unwrap_or("unknown");
    log::info!("📨 [BIOMETRIC] Command result from SN={}: {}", sn, body.trim());

    // Parse command ID from result and update status
    // Format: "ID=xxx&Return=0" or similar
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return HttpResponse::Ok().body("OK"),
    };

    // Try to extract command ID
    for part in body.split('&') {
        if part.starts_with("ID=") {
            if let Ok(cmd_id) = part[3..].parse::<i64>() {
                let _ = conn.execute(
                    "UPDATE biometric_commands SET status='executed', result=?1 WHERE id=?2",
                    rusqlite::params![&body, cmd_id],
                );
            }
        }
    }

    HttpResponse::Ok().content_type("text/plain").body("OK")
}

// ═══════════════════════════════════════════════════════════════════
//  Punch → Attendance Resolution Logic
// ═══════════════════════════════════════════════════════════════════

/// Toggle check-in/out from the last stored punch when device status is unreliable.
pub fn next_punch_type(conn: &rusqlite::Connection, device_serial: &str, device_pin: &str) -> i64 {
    let last: Option<i64> = conn
        .query_row(
            "SELECT punch_type FROM biometric_punches
             WHERE device_serial=?1 AND device_pin=?2
             ORDER BY punch_time DESC, id DESC LIMIT 1",
            rusqlite::params![device_serial, device_pin],
            |row| row.get(0),
        )
        .ok();
    match last {
        Some(0) => 1,
        _ => 0,
    }
}

/// Resolve a stored punch into attendance and mark it processed.
pub fn process_punch_to_attendance(
    conn: &rusqlite::Connection,
    punch_id: i64,
    user_id: i64,
    timestamp: &str,
    status: i64,
) {
    resolve_punch_to_attendance(conn, user_id, timestamp, status);
    let _ = conn.execute(
        "UPDATE biometric_punches SET is_processed=1 WHERE id=?1",
        [punch_id],
    );
}

fn resolve_punch_to_attendance(conn: &rusqlite::Connection, user_id: i64, timestamp: &str, status: i64) {
    use crate::attendance_logic::{close_open_session_before_clock_in, find_open_attendance_session};
    use crate::shift_logic::{
        calc_duration_minutes, is_early_departure, is_late_arrival, resolve_shift_for_user,
    };

    // Extract date from timestamp (e.g., "2026-06-03 09:15:22" → "2026-06-03")
    let date = if timestamp.len() >= 10 { &timestamp[..10] } else { return; };
    let time = if timestamp.len() >= 19 { &timestamp[11..19] } else { return; };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let shift = resolve_shift_for_user(conn, user_id, date);

    // Active session: same-day first, then any open session (overnight / night shift)
    let active_session = find_open_attendance_session(conn, user_id, date);

    let insert_clock_in = |is_late: bool| {
        let _ = conn.execute(
            "INSERT INTO attendance (user_id, date, clock_in, status, is_late, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'present', ?4, 'biometric', ?5, ?5)",
            rusqlite::params![user_id, date, time, if is_late { 1 } else { 0 }, &now],
        );
    };

    let clock_out_session = |att_id: i64, session_date: &str| {
        let session_shift = resolve_shift_for_user(conn, user_id, session_date);
        let clock_in: String = conn
            .query_row(
                "SELECT clock_in FROM attendance WHERE id=?1",
                [att_id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let duration = calc_duration_minutes(&clock_in, time);
        let early = is_early_departure(
            time,
            &session_shift.start_time,
            &session_shift.end_time,
            session_shift.grace_out_minutes,
        );
        let _ = conn.execute(
            "UPDATE attendance SET clock_out=?1, duration_minutes=?2, is_early_exit=?3, updated_at=?4 WHERE id=?5",
            rusqlite::params![time, duration, if early { 1 } else { 0 }, &now, att_id],
        );
    };

    match (status, active_session) {
        // Check-in: close any open session first (shift-aware), then start a new one
        (0, Some((att_id, _, _))) => {
            close_open_session_before_clock_in(conn, user_id, date, time, &now, &shift);
            let is_late = is_late_arrival(time, &shift.start_time, &shift.end_time, shift.grace_in_minutes);
            insert_clock_in(is_late);
            log::info!(
                "  ✅ Clock-IN (new session) for user_id={} at {} closed={} shift={:?} late={}",
                user_id,
                timestamp,
                att_id,
                shift.template_name,
                is_late
            );
        }
        (0, None) => {
            let is_late = is_late_arrival(time, &shift.start_time, &shift.end_time, shift.grace_in_minutes);
            insert_clock_in(is_late);
            log::info!("  ✅ Clock-IN created for user_id={} at {}", user_id, timestamp);
        }
        // Explicit check-out with an open session (including overnight from prior day)
        (1, Some((att_id, session_date, _))) => {
            clock_out_session(att_id, &session_date);
            log::info!(
                "  ✅ Clock-OUT for user_id={} at {} (session date={})",
                user_id,
                timestamp,
                session_date
            );
        }
        (1, None) => {
            log::warn!(
                "  ⚠️ Orphan check-out ignored for user_id={} at {} — no open session",
                user_id,
                timestamp
            );
        }
        // Non check-in punch while session is open → clock out (device sometimes sends status 2+)
        (_, Some((att_id, session_date, _))) => {
            clock_out_session(att_id, &session_date);
            log::info!(
                "  ✅ Clock-OUT (status={}) for user_id={} at {} (session date={})",
                status,
                user_id,
                timestamp,
                session_date
            );
        }
        (_, None) => {
            log::warn!(
                "  ⚠️ Unhandled punch status={} for user_id={} at {} — no open session",
                status,
                user_id,
                timestamp
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Admin API Endpoints (Authenticated)
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/biometric/devices — List all registered devices
pub async fn devices_list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let mut stmt = conn.prepare(
        "SELECT * FROM biometric_devices ORDER BY created_at DESC"
    ).unwrap();
    let items: Vec<BiometricDevice> = stmt.query_map([], BiometricDevice::from_row)
        .unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(ApiResponse::success(items))
}

/// POST /api/admin/biometric/devices — Register a device manually
pub async fn devices_store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<serde_json::Value>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let sn = body.get("serial_number").and_then(|v| v.as_str()).unwrap_or("");
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("BIO-PARK D01");
    let location = body.get("location").and_then(|v| v.as_str()).unwrap_or("");
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if sn.is_empty() {
        return HttpResponse::BadRequest().json(ApiError::new("serial_number is required"));
    }

    match conn.execute(
        "INSERT INTO biometric_devices (serial_number, name, location, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        rusqlite::params![sn, name, location, &now],
    ) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Device registered"}))),
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed: {}", e))),
    }
}

/// DELETE /api/admin/biometric/devices/{id}
pub async fn devices_destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let id = path.into_inner();
    let _ = conn.execute("DELETE FROM biometric_devices WHERE id=?1", [id]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Device deleted"})))
}

/// GET /api/admin/biometric/punches — List raw punch logs
pub async fn punches_list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let mut stmt = conn.prepare(
        "SELECT bp.*, u.name as user_name FROM biometric_punches bp
         LEFT JOIN users u ON bp.user_id = u.id
         ORDER BY bp.punch_time DESC LIMIT 200"
    ).unwrap();

    let items: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>("id")?,
            "device_serial": row.get::<_, String>("device_serial")?,
            "device_pin": row.get::<_, String>("device_pin")?,
            "punch_time": row.get::<_, String>("punch_time")?,
            "punch_type": row.get::<_, i64>("punch_type")?,
            "verify_method": row.get::<_, i64>("verify_method")?,
            "user_id": row.get::<_, Option<i64>>("user_id")?,
            "user_name": row.get::<_, Option<String>>("user_name").unwrap_or(None),
            "is_processed": row.get::<_, i64>("is_processed")?,
            "created_at": row.get::<_, Option<String>>("created_at")?,
        }))
    }).unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(ApiResponse::success(items))
}

/// GET /api/admin/biometric/mapping — List user-device mappings
pub async fn mapping_list(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let mut stmt = conn.prepare(
        "SELECT bm.*, u.name as user_name FROM biometric_user_map bm
         LEFT JOIN users u ON bm.user_id = u.id
         ORDER BY bm.device_serial, bm.device_pin"
    ).unwrap();

    let items: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>("id")?,
            "device_serial": row.get::<_, String>("device_serial")?,
            "device_pin": row.get::<_, String>("device_pin")?,
            "user_id": row.get::<_, i64>("user_id")?,
            "user_name": row.get::<_, Option<String>>("user_name").unwrap_or(None),
            "created_at": row.get::<_, Option<String>>("created_at")?,
        }))
    }).unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(ApiResponse::success(items))
}

/// POST /api/admin/biometric/mapping — Create a user-device PIN mapping
pub async fn mapping_store(pool: web::Data<DbPool>, req: HttpRequest, body: web::Json<UserMapRequest>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    match conn.execute(
        "INSERT INTO biometric_user_map (device_serial, device_pin, user_id, created_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(device_serial, device_pin) DO UPDATE SET user_id=?3",
        rusqlite::params![&body.device_serial, &body.device_pin, body.user_id, &now],
    ) {
        Ok(_) => {
            // Retroactively resolve any unprocessed punches for this PIN
            let punch_count = retroactive_resolve(&conn, &body.device_serial, &body.device_pin, body.user_id);
            HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                "message": "Mapping saved",
                "retroactive_punches": punch_count,
            })))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiError::new(&format!("Failed: {}", e))),
    }
}

/// DELETE /api/admin/biometric/mapping/{id}
pub async fn mapping_destroy(pool: web::Data<DbPool>, req: HttpRequest, path: web::Path<i64>) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };
    let id = path.into_inner();
    let _ = conn.execute("DELETE FROM biometric_user_map WHERE id=?1", [id]);
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({"message": "Mapping deleted"})))
}

/// GET /api/admin/biometric/stats — Dashboard statistics
pub async fn biometric_stats(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let _c = match get_claims_from_request(&req) { Ok(c) => c, Err(e) => return HttpResponse::Unauthorized().json(ApiError::new(&e.to_string())) };
    let conn = match pool.get() { Ok(c) => c, Err(_) => return HttpResponse::InternalServerError().json(ApiError::new("DB error")) };

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let total_devices: i64 = conn.query_row("SELECT COUNT(*) FROM biometric_devices", [], |r| r.get(0)).unwrap_or(0);
    let active_devices: i64 = conn.query_row(
        "SELECT COUNT(*) FROM biometric_devices
         WHERE last_heartbeat IS NOT NULL
         AND last_heartbeat >= datetime('now', '-10 minutes')",
        [],
        |r| r.get(0),
    ).unwrap_or(0);
    let today_punches: i64 = conn.query_row(
        "SELECT COUNT(*) FROM biometric_punches WHERE punch_time LIKE ?1 || '%'",
        [&today], |r| r.get(0)
    ).unwrap_or(0);
    let total_mappings: i64 = conn.query_row("SELECT COUNT(*) FROM biometric_user_map", [], |r| r.get(0)).unwrap_or(0);
    let unmapped_punches: i64 = conn.query_row(
        "SELECT COUNT(*) FROM biometric_punches WHERE user_id IS NULL",
        [], |r| r.get(0)
    ).unwrap_or(0);

    // Last heartbeat info
    let last_heartbeat: Option<String> = conn.query_row(
        "SELECT last_heartbeat FROM biometric_devices ORDER BY last_heartbeat DESC LIMIT 1",
        [], |r| r.get(0)
    ).ok();

    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "total_devices": total_devices,
        "active_devices": active_devices,
        "today_punches": today_punches,
        "total_mappings": total_mappings,
        "unmapped_punches": unmapped_punches,
        "last_heartbeat": last_heartbeat,
    })))
}

fn ws_token_from_query(req: &HttpRequest, jwt_secret: &str) -> Result<(), Error> {
    let token = req
        .uri()
        .query()
        .and_then(|q| {
            url_query_param(q, "token")
        })
        .ok_or_else(|| ErrorUnauthorized("Missing token query parameter"))?;

    let validation = Validation::new(Algorithm::HS256);
    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| ErrorUnauthorized("Invalid or expired token"))?;
    Ok(())
}

fn url_query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        if parts.next()? == key {
            parts.next()
        } else {
            None
        }
    })
}

/// GET /api/admin/biometric/ws?token=JWT — live updates for admin UI (reconnects like a persistent channel).
pub async fn biometric_live_ws(
    req: HttpRequest,
    stream: web::Payload,
    events: web::Data<BiometricEvents>,
    jwt: web::Data<Arc<String>>,
) -> Result<HttpResponse, Error> {
    ws_token_from_query(&req, jwt.as_str())?;

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let mut rx = events.subscribe();

    actix_web::rt::spawn(async move {
        let welcome = serde_json::json!({
            "type": "connected",
            "message": "Biometric live stream active",
        });
        if session
            .text(serde_json::to_string(&welcome).unwrap_or_default())
            .await
            .is_err()
        {
            return;
        }

        loop {
            tokio::select! {
                incoming = msg_stream.next() => {
                    match incoming {
                        Some(Ok(Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Err(_)) => break,
                        _ => {}
                    }
                }
                evt = rx.recv() => {
                    match evt {
                        Ok(payload) => {
                            if session.text(payload).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            }
        }
        let _ = session.close(None).await;
    });

    Ok(response)
}

// ═══════════════════════════════════════════════════════════════════
//  Helper: Retroactive resolution for newly-mapped PINs
// ═══════════════════════════════════════════════════════════════════

fn retroactive_resolve(conn: &rusqlite::Connection, device_serial: &str, device_pin: &str, user_id: i64) -> i64 {
    // Update all unprocessed punches for this PIN with the user_id
    let _ = conn.execute(
        "UPDATE biometric_punches SET user_id=?1 WHERE device_serial=?2 AND device_pin=?3 AND user_id IS NULL",
        rusqlite::params![user_id, device_serial, device_pin],
    );

    // Get all unprocessed punches for this user, ordered by time
    let mut stmt = match conn.prepare(
        "SELECT id, punch_time, punch_type FROM biometric_punches
         WHERE device_serial=?1 AND device_pin=?2 AND is_processed=0
         ORDER BY punch_time ASC"
    ) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let punches: Vec<(i64, String, i64)> = stmt.query_map(
        rusqlite::params![device_serial, device_pin],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap().filter_map(|r| r.ok()).collect();

    let mut count: i64 = 0;
    for (punch_id, timestamp, status) in &punches {
        resolve_punch_to_attendance(conn, user_id, timestamp, *status);
        let _ = conn.execute("UPDATE biometric_punches SET is_processed=1 WHERE id=?1", [punch_id]);
        count += 1;
    }

    count
}

// ═══════════════════════════════════════════════════════════════════
//  M-CARD / BIO-PARK ADMS Protocol (/pub/chat)
//  The device sends GET /pub/chat for handshake and POST /pub/chat
//  to push attendance data. Query params carry SN, options, etc.
// ═══════════════════════════════════════════════════════════════════

/// Query params the M-CARD ADMS device sends on /pub/chat
#[derive(Debug, serde::Deserialize)]
pub struct AdmsQuery {
    #[serde(rename = "SN", alias = "sn")]
    pub sn: Option<String>,
    #[serde(rename = "INFO", alias = "info")]
    pub info: Option<String>,
    #[serde(rename = "TABLE", alias = "table")]
    pub table: Option<String>,
    #[serde(rename = "STAMP", alias = "stamp")]
    pub stamp: Option<String>,
    #[serde(rename = "Ession", alias = "ession", alias = "session")]
    pub session: Option<String>,
}

/// GET /pub/chat — WebSocket endpoint for BIO-PARK device communication.
/// The device requests a WebSocket upgrade here. All attendance data and
/// heartbeats flow through this persistent WS connection.
pub async fn adms_chat_ws(
    pool: web::Data<DbPool>,
    events: web::Data<BiometricEvents>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let peer_ip = req.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    log::info!("🔗 [ADMS-WS] WebSocket upgrade from {}", peer_ip);

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let pool = pool.clone();
    let events = events.clone();

    actix_web::rt::spawn(async move {
        log::info!("✅ [ADMS-WS] Connection established with {}", peer_ip);

        while let Some(msg) = msg_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let text_str = text.to_string();
                    log::info!("📨 [ADMS-WS] Text from {}: {}", peer_ip, text_str);

                    // Parse the device message — may produce multiple responses
                    let responses = handle_adms_ws_message(&pool, &events, &text_str, &peer_ip);
                    for resp in responses {
                        log::info!("📤 [ADMS-WS] Sending to {}: {}", peer_ip, resp);
                        if session.text(resp).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(Message::Binary(bin)) => {
                    log::info!("📨 [ADMS-WS] Binary ({} bytes) from {}", bin.len(), peer_ip);
                    if session.text("OK").await.is_err() {
                        break;
                    }
                }
                Ok(Message::Ping(bytes)) => {
                    if session.pong(&bytes).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Pong(_)) => {}
                Ok(Message::Close(_)) => {
                    log::info!("🔌 [ADMS-WS] Device {} closed connection", peer_ip);
                    break;
                }
                Ok(Message::Continuation(_)) => {}
                Ok(Message::Nop) => {}
                Err(e) => {
                    log::error!("❌ [ADMS-WS] Error from {}: {}", peer_ip, e);
                    break;
                }
            }
        }

        log::info!("🔌 [ADMS-WS] Session ended for {}", peer_ip);
        let _ = session.close(None).await;
    });

    Ok(response)
}

/// Parse a WebSocket text message from the BIO-PARK device and return JSON responses.
/// The device sends JSON messages with a "cmd" field.
/// Returns multiple messages (e.g., reg ack + getlog request).
fn handle_adms_ws_message(
    pool: &web::Data<DbPool>,
    events: &web::Data<BiometricEvents>,
    text: &str,
    ip: &str,
) -> Vec<String> {
    // Try to parse as JSON
    let msg: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            log::warn!("[ADMS-WS] Non-JSON message from {}: {}", ip, text);
            return vec![serde_json::json!({"ret":"OK"}).to_string()];
        }
    };

    let cmd = msg.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
    let sn = msg.get("sn").and_then(|v| v.as_str()).unwrap_or("unknown");

    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return vec![serde_json::json!({"ret":"ERR","reason":"DB"}).to_string()],
    };

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    match cmd {
        // ── Device Registration ──────────────────────────────────
        "reg" => {
            log::info!("✅ [ADMS-WS] Device registered: SN={} IP={}", sn, ip);

            let devinfo = msg.get("devinfo");
            let model = devinfo.and_then(|d| d.get("modelname")).and_then(|v| v.as_str()).unwrap_or("BIO-PARK");
            let firmware = devinfo.and_then(|d| d.get("firmware")).and_then(|v| v.as_str()).unwrap_or("");
            let mac = devinfo.and_then(|d| d.get("mac")).and_then(|v| v.as_str()).unwrap_or("");
            let new_logs = devinfo.and_then(|d| d.get("usednewlog")).and_then(|v| v.as_i64()).unwrap_or(0);

            // Upsert device
            let _ = conn.execute(
                "INSERT INTO biometric_devices (serial_number, name, ip_address, last_heartbeat, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?4, ?4)
                 ON CONFLICT(serial_number) DO UPDATE SET name=?2, ip_address=?3, last_heartbeat=?4, is_active=1, updated_at=?4",
                rusqlite::params![sn, format!("{} ({})", model, firmware), ip, &now],
            );

            events.emit(
                "device_online",
                serde_json::json!({
                    "serial_number": sn,
                    "ip_address": ip,
                    "model": model,
                    "firmware": firmware,
                    "mac": mac,
                    "new_logs": new_logs,
                    "last_heartbeat": now,
                }),
            );

            // Build response: ACK the registration with options.
            // cloudtime must match device timezone (UTC+5:30 IST)
            let ist_offset = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
            let cloudtime = chrono::Utc::now().with_timezone(&ist_offset)
                .format("%Y-%m-%d %H:%M:%S").to_string();

            // result:true is CRITICAL — device won't proceed without it!
            // nosenduser=0, nosendlog=0: tell device to push both users and logs
            // realtime=1: push attendance events in real-time
            let responses = vec![
                serde_json::json!({
                    "ret": "reg",
                    "result": true,
                    "cloudtime": cloudtime,
                    "nosenduser": 0,
                    "nosendlog": 0,
                    "transinterval": 1,
                    "transtimes": "00:00;14:05",
                    "realtime": 1,
                    "encrypt": 0
                }).to_string(),
            ];

            responses
        }

        // ── Attendance Log Push ──────────────────────────────────
        "sendlog" => {
            // record can be a single object OR an array of objects
            let records: Vec<&serde_json::Value> = match msg.get("record") {
                Some(serde_json::Value::Array(arr)) => arr.iter().collect(),
                Some(obj @ serde_json::Value::Object(_)) => vec![obj],
                _ => vec![],
            };

            let total = records.len();
            let mut stored = 0;

            for rec in &records {
                // enrollid can be a number or string
                let pin = rec.get("enrollid")
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => String::new(),
                    })
                    .or_else(|| rec.get("pin").and_then(|v| v.as_str()).map(String::from))
                    .unwrap_or_default();
                let timestamp = rec.get("time").and_then(|v| v.as_str()).unwrap_or("");
                let status: i64 = rec.get("mode").and_then(|v| v.as_i64())
                    .or_else(|| rec.get("status").and_then(|v| v.as_i64()))
                    .unwrap_or(0);
                let verify: i64 = rec.get("type").and_then(|v| v.as_i64())
                    .or_else(|| rec.get("verify").and_then(|v| v.as_i64()))
                    .unwrap_or(0);
                let inout: i64 = rec.get("inout").and_then(|v| v.as_i64()).unwrap_or(0);

                if pin.is_empty() || timestamp.is_empty() { continue; }

                log::info!("📋 [ADMS-WS] Punch: SN={} PIN={} Time={} Mode={} InOut={}",
                    sn, pin, timestamp, status, inout);

                let user_id: Option<i64> = conn.query_row(
                    "SELECT user_id FROM biometric_user_map WHERE device_serial=?1 AND device_pin=?2",
                    rusqlite::params![sn, &pin],
                    |row| row.get(0),
                ).ok();

                let Some(uid) = user_id else {
                    log::info!("  ⏭️ [ADMS-WS] Unmapped PIN={} on SN={} — punch ignored", pin, sn);
                    continue;
                };

                let exists: bool = conn.query_row(
                    "SELECT COUNT(*) FROM biometric_punches WHERE device_serial=?1 AND device_pin=?2 AND punch_time=?3",
                    rusqlite::params![sn, &pin, timestamp],
                    |row| row.get::<_, i64>(0),
                ).unwrap_or(0) > 0;

                if !exists {
                    if conn.execute(
                        "INSERT INTO biometric_punches (device_serial, device_pin, punch_time, punch_type, verify_method, user_id, is_processed, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
                        rusqlite::params![sn, &pin, timestamp, inout, verify, uid, &now],
                    ).is_ok() {
                        let punch_id = conn.last_insert_rowid();
                        process_punch_to_attendance(&conn, punch_id, uid, timestamp, inout);
                        stored += 1;
                    }
                }
            }

            if stored > 0 {
                log::info!("✅ [ADMS-WS] Stored {}/{} punches from SN={}", stored, total, sn);
                events.emit(
                    "punches_received",
                    serde_json::json!({
                        "serial_number": sn,
                        "count": stored,
                    }),
                );
            }

            // ACK — count must match what the device sent
            vec![serde_json::json!({"ret":"sendlog","result":true,"count":total,"logindex":0}).to_string()]
        }

        // ── User data from device ────────────────────────────────
        "senduser" => {
            log::info!("👤 [ADMS-WS] User data from SN={}: {}", sn, text);
            vec![serde_json::json!({"ret":"senduser","result":1,"count":1}).to_string()]
        }

        // ── Heartbeat / keep-alive ───────────────────────────────
        "heartbeat" | "ping" => {
            record_device_touch(&conn, events, sn, ip, "device_heartbeat");
            vec![serde_json::json!({"ret":"heartbeat","cloudtime": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()}).to_string()]
        }

        // ── Unknown command ──────────────────────────────────────
        _ => {
            log::info!("❓ [ADMS-WS] Unknown cmd='{}' from SN={}: {}", cmd, sn, text);
            vec![serde_json::json!({"ret": cmd, "result": 1}).to_string()]
        }
    }
}



/// POST /pub/chat — Device pushes attendance logs.
/// Body contains tab-separated ATTLOG records, one per line.
pub async fn adms_chat_post(
    pool: web::Data<DbPool>,
    events: web::Data<BiometricEvents>,
    query: web::Query<AdmsQuery>,
    req: HttpRequest,
    body: String,
) -> HttpResponse {
    let sn = query.sn.clone().unwrap_or_else(|| "unknown".into());
    let table = query.table.as_deref().unwrap_or("ATTLOG");
    let peer_ip = req.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();

    log::info!(
        "📥 [ADMS] POST /pub/chat — SN={} table={} body_len={} IP={}",
        sn, table, body.len(), peer_ip
    );
    log::info!("📥 [ADMS] Body:\n{}", body);

    if table.eq_ignore_ascii_case("ATTLOG") {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return HttpResponse::InternalServerError().body("ERR: DB"),
        };

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut processed = 0;

        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 2 { continue; }

            let pin = fields[0].trim();
            let timestamp = fields.get(1).map(|s| s.trim()).unwrap_or("");
            let status: i64 = fields.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            let verify: i64 = fields.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(0);

            if pin.is_empty() || timestamp.is_empty() { continue; }

            log::info!("  📋 [ADMS] Punch: PIN={} Time={} Status={} Verify={}", pin, timestamp, status, verify);

            let user_id: Option<i64> = conn.query_row(
                "SELECT user_id FROM biometric_user_map WHERE device_serial=?1 AND device_pin=?2",
                rusqlite::params![&sn, pin],
                |row| row.get(0),
            ).ok();

            let Some(uid) = user_id else {
                log::info!("  ⏭️  [ADMS] Unmapped PIN={} on SN={} — punch ignored", pin, sn);
                continue;
            };

            // Duplicate check
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) FROM biometric_punches WHERE device_serial=?1 AND device_pin=?2 AND punch_time=?3",
                rusqlite::params![&sn, pin, timestamp],
                |row| row.get::<_, i64>(0),
            ).unwrap_or(0) > 0;

            if exists {
                log::info!("  ⏭️  Duplicate punch skipped: PIN={} Time={}", pin, timestamp);
                continue;
            }

            if conn.execute(
                "INSERT INTO biometric_punches (device_serial, device_pin, punch_time, punch_type, verify_method, user_id, is_processed, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
                rusqlite::params![&sn, pin, timestamp, status, verify, uid, &now],
            ).is_ok() {
                let punch_id = conn.last_insert_rowid();
                process_punch_to_attendance(&conn, punch_id, uid, timestamp, status);
                processed += 1;
            }
        }

        // Update heartbeat
        let _ = conn.execute(
            "UPDATE biometric_devices SET last_heartbeat=?1, updated_at=?1 WHERE serial_number=?2",
            rusqlite::params![&now, &sn],
        );

        log::info!("✅ [ADMS] Processed {} punches from SN={}", processed, sn);
        if processed > 0 {
            events.emit(
                "punches_received",
                serde_json::json!({
                    "serial_number": sn,
                    "count": processed,
                }),
            );
        }
    } else {
        log::info!("ℹ️  [ADMS] Ignoring table={} from SN={}", table, sn);
    }

    HttpResponse::Ok().content_type("text/plain").body("OK")
}

/// GET /pub/getrequest — Device polls for pending commands (ADMS variant)
pub async fn adms_getrequest(
    pool: web::Data<DbPool>,
    events: web::Data<BiometricEvents>,
    query: web::Query<AdmsQuery>,
    req: HttpRequest,
) -> HttpResponse {
    // Reuse the iClock getrequest logic
    let sn = query.sn.clone().unwrap_or_else(|| "unknown".into());
    let peer_ip = req.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();

    if let Ok(conn) = pool.get() {
        record_device_touch(&conn, &events, &sn, &peer_ip, "device_heartbeat");

        let cmd: Option<(i64, String)> = conn.query_row(
            "SELECT id, command FROM biometric_commands WHERE device_serial=?1 AND status='pending' ORDER BY id LIMIT 1",
            rusqlite::params![&sn],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).ok();

        if let Some((cmd_id, command)) = cmd {
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let _ = conn.execute(
                "UPDATE biometric_commands SET status='sent', executed_at=?1 WHERE id=?2",
                rusqlite::params![&now, cmd_id],
            );
            return HttpResponse::Ok().content_type("text/plain").body(format!("C:{}:{}", cmd_id, command));
        }
    }

    HttpResponse::Ok().content_type("text/plain").body("OK")
}
