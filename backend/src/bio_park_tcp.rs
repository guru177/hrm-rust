//! BIO-PARK D01 binary TCP protocol handler.
//!
//! The device speaks a proprietary binary protocol over raw TCP (NOT HTTP).
//! Packet format:
//!   [0..2]  Magic: 0xA5 0x5A
//!   [2..4]  Command ID (little-endian u16)
//!   [4..8]  Session / sequence bytes
//!   [8..]   Payload (variable length)
//!
//! Known commands:
//!   0x0001 — Heartbeat / Connect (device → server). Payload contains serial number at offset 8+8.
//!   0x07D0 — ACK (server → device). Echo session bytes back.
//!   0x01F4 — REG_EVENT (server → device). Register for real-time punch events.
//!   Other  — Potential real-time attendance events from device.

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::biometric_events::BiometricEvents;
use crate::db::DbPool;

const MAGIC: [u8; 2] = [0xA5, 0x5A];
const ACK_CMD: [u8; 2] = [0xD0, 0x07];

/// Run the raw TCP listener that speaks the BIO-PARK binary protocol.
pub async fn run(
    host: &str,
    port: u16,
    pool: Arc<DbPool>,
    events: Arc<BiometricEvents>,
) -> std::io::Result<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    log::info!("📡 BIO-PARK binary TCP listener on {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer)) => {
                let pool = pool.clone();
                let events = events.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_device(socket, peer, pool, events).await {
                        log::error!("[BIO-PARK] session error from {}: {}", peer, e);
                    }
                });
            }
            Err(e) => log::error!("[BIO-PARK] accept error: {}", e),
        }
    }
}

async fn handle_device(
    mut socket: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
    pool: Arc<DbPool>,
    events: Arc<BiometricEvents>,
) -> std::io::Result<()> {
    log::info!("🔗 [BIO-PARK] Device connected from {}", peer);

    let mut buf = vec![0u8; 4096];
    let mut registered_for_events = false;
    let peer_ip = peer.ip().to_string();

    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            log::info!("🔌 [BIO-PARK] Device {} disconnected", peer);
            return Ok(());
        }

        let data = &buf[..n];

        // Validate magic header
        if n < 8 || data[0] != MAGIC[0] || data[1] != MAGIC[1] {
            log::warn!(
                "[BIO-PARK] Invalid packet ({} bytes) from {}: {}",
                n, peer, to_hex(&data[..n.min(48)])
            );
            continue;
        }

        let cmd = u16::from_le_bytes([data[2], data[3]]);
        let session = &data[4..8];

        match cmd {
            // ── Heartbeat / Connect ──────────────────────────────
            0x0001 => {
                let sn = extract_serial(&data[8..n]);
                log::info!("💓 [BIO-PARK] Heartbeat SN={} from {}", sn, peer);

                // ACK the heartbeat
                send_ack(&mut socket, session).await?;

                // First heartbeat → register for real-time events
                if !registered_for_events {
                    send_reg_event(&mut socket, session).await?;
                    registered_for_events = true;
                    log::info!("📤 [BIO-PARK] Sent REG_EVENT to SN={}", sn);
                }

                // Update device record in DB
                touch_device(&pool, &events, &sn, &peer_ip);
            }

            // ── Any other command (potential punch event) ────────
            other => {
                let payload = &data[8..n];
                log::info!(
                    "📦 [BIO-PARK] CMD=0x{:04X} ({}) from {} | {} bytes payload: {}",
                    other, other, peer, payload.len(), to_hex(payload)
                );

                // ACK so the device doesn't retry
                send_ack(&mut socket, session).await?;

                // Try parsing as a real-time attendance event
                try_parse_punch(&pool, &events, other, payload, &peer_ip);
            }
        }
    }
}

// ─── Protocol helpers ────────────────────────────────────────────

async fn send_ack(
    socket: &mut tokio::net::TcpStream,
    session: &[u8],
) -> std::io::Result<()> {
    let mut pkt = Vec::with_capacity(8);
    pkt.extend_from_slice(&MAGIC);
    pkt.extend_from_slice(&ACK_CMD);
    pkt.extend_from_slice(session);
    socket.write_all(&pkt).await
}

async fn send_reg_event(
    socket: &mut tokio::net::TcpStream,
    session: &[u8],
) -> std::io::Result<()> {
    let mut pkt = Vec::with_capacity(12);
    pkt.extend_from_slice(&MAGIC);
    pkt.extend_from_slice(&[0xF4, 0x01]); // CMD 500 = 0x01F4 LE
    pkt.extend_from_slice(session);
    pkt.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // event mask
    socket.write_all(&pkt).await
}

/// Extract the ASCII serial number from the heartbeat payload.
/// Layout: 8 bytes of counters, then the serial string (null-padded).
fn extract_serial(payload: &[u8]) -> String {
    if payload.len() < 9 {
        return "unknown".into();
    }
    // Serial starts at byte 8 of the payload (offset 16 overall)
    let sn_start = 8;
    let sn_bytes = &payload[sn_start..];
    let end = sn_bytes
        .iter()
        .position(|&b| b == 0 || !b.is_ascii_graphic())
        .unwrap_or(sn_bytes.len());
    if end == 0 {
        "unknown".into()
    } else {
        String::from_utf8_lossy(&sn_bytes[..end]).into_owned()
    }
}

/// Attempt to parse an unknown command as a punch/attendance event and store it.
fn try_parse_punch(
    pool: &Arc<DbPool>,
    events: &Arc<BiometricEvents>,
    cmd: u16,
    payload: &[u8],
    ip: &str,
) {
    let ascii_parts: String = payload
        .iter()
        .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
        .collect();
    log::info!("  🔍 [BIO-PARK] ASCII view: {}", ascii_parts);

    if payload.len() < 4 {
        return;
    }

    let Ok(conn) = pool.get() else {
        return;
    };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let punch_time = extract_timestamp_from_payload(payload).unwrap_or_else(|| now.clone());

    let sn: String = conn
        .query_row(
            "SELECT serial_number FROM biometric_devices WHERE ip_address=?1 ORDER BY last_heartbeat DESC LIMIT 1",
            rusqlite::params![ip],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "unknown".into());

    let pin = extract_pin_from_payload(payload);
    if pin.is_empty() {
        return;
    }

    log::info!("  👤 [BIO-PARK] Detected PIN={} from CMD=0x{:04X}", pin, cmd);

    let user_id: Option<i64> = conn
        .query_row(
            "SELECT user_id FROM biometric_user_map WHERE device_serial=?1 AND device_pin=?2",
            rusqlite::params![&sn, &pin],
            |row| row.get(0),
        )
        .ok();

    let Some(uid) = user_id else {
        log::info!("  ⏭️ [BIO-PARK] Unmapped PIN={} on SN={} — punch ignored", pin, sn);
        return;
    };

    let exact_dup: bool = conn
        .query_row(
            "SELECT 1 FROM biometric_punches
             WHERE device_serial=?1 AND device_pin=?2 AND punch_time=?3 LIMIT 1",
            rusqlite::params![&sn, &pin, &punch_time],
            |_| Ok(()),
        )
        .is_ok();
    if exact_dup {
        log::info!("  ⏭️ [BIO-PARK] Duplicate punch skipped PIN={} at {}", pin, punch_time);
        return;
    }

    let recent_dup: bool = conn
        .query_row(
            "SELECT 1 FROM biometric_punches
             WHERE device_serial=?1 AND device_pin=?2
               AND punch_time >= datetime(?3, '-60 seconds')
             LIMIT 1",
            rusqlite::params![&sn, &pin, &punch_time],
            |_| Ok(()),
        )
        .is_ok();
    if recent_dup {
        log::info!("  ⏭️ [BIO-PARK] Near-duplicate punch skipped PIN={}", pin);
        return;
    }

    let punch_type = crate::handlers::biometric::next_punch_type(&conn, &sn, &pin);

    if conn
        .execute(
            "INSERT INTO biometric_punches (device_serial, device_pin, punch_time, punch_type, verify_method, user_id, is_processed, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, 0, ?6)",
            rusqlite::params![&sn, &pin, &punch_time, punch_type, uid, &now],
        )
        .is_ok()
    {
        let punch_id = conn.last_insert_rowid();
        crate::handlers::biometric::process_punch_to_attendance(
            &conn, punch_id, uid, &punch_time, punch_type,
        );
    }

    events.emit(
        "punches_received",
        serde_json::json!({
            "serial_number": sn,
            "pin": pin,
            "count": 1,
        }),
    );
}

/// Try to extract a device timestamp from the binary payload.
fn extract_timestamp_from_payload(payload: &[u8]) -> Option<String> {
    let text: String = payload
        .iter()
        .filter(|&&b| b.is_ascii())
        .map(|&b| b as char)
        .collect();
    for i in 0..text.len().saturating_sub(18) {
        let chunk = &text[i..i + 19];
        if chrono::NaiveDateTime::parse_from_str(chunk, "%Y-%m-%d %H:%M:%S").is_ok() {
            return Some(chunk.to_string());
        }
    }
    None
}

/// Try to find a numeric PIN string within the binary payload.
fn extract_pin_from_payload(payload: &[u8]) -> String {
    // Scan for the longest run of ASCII digits
    let mut best = String::new();
    let mut current = String::new();
    for &b in payload {
        if b.is_ascii_digit() {
            current.push(b as char);
        } else {
            if current.len() > best.len() {
                best = current.clone();
            }
            current.clear();
        }
    }
    if current.len() > best.len() {
        best = current;
    }
    // Only return if it looks like a reasonable PIN (1-20 digits)
    if best.len() >= 1 && best.len() <= 20 {
        best
    } else {
        String::new()
    }
}

fn touch_device(pool: &Arc<DbPool>, events: &Arc<BiometricEvents>, sn: &str, ip: &str) {
    if let Ok(conn) = pool.get() {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = conn.execute(
            "INSERT INTO biometric_devices (serial_number, ip_address, last_heartbeat, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, ?3, ?3)
             ON CONFLICT(serial_number) DO UPDATE SET ip_address=?2, last_heartbeat=?3, is_active=1, updated_at=?3",
            rusqlite::params![sn, ip, &now],
        );
        events.emit(
            "device_heartbeat",
            serde_json::json!({
                "serial_number": sn,
                "ip_address": ip,
                "last_heartbeat": now,
            }),
        );
    }
}

/// Format bytes as hex string for logging.
fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("")
}
