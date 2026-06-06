/// Application configuration loaded from environment variables.
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    /// Dedicated port for BIO-PARK / ZKTeco iClock devices (default 7788).
    pub biometric_port: u16,
    /// Raw TCP port for BIO-PARK binary protocol (default 5010).
    pub bio_park_tcp_port: u16,
    pub database_path: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    /// Shared secret for inbound webhooks (e.g. resume ingestion). Empty = webhook disabled.
    pub webhook_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            // 0.0.0.0 required so BIO-PARK / iClock devices on the LAN can reach /iclock/*
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .expect("PORT must be a number"),
            biometric_port: std::env::var("BIOMETRIC_PORT")
                .unwrap_or_else(|_| "7788".to_string())
                .parse()
                .expect("BIOMETRIC_PORT must be a number"),
            bio_park_tcp_port: std::env::var("BIO_PARK_TCP_PORT")
                .unwrap_or_else(|_| "5010".to_string())
                .parse()
                .expect("BIO_PARK_TCP_PORT must be a number"),
            database_path: std::env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "../database/database.sqlite".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "hrm-super-secret-key-change-in-production-2026".to_string()),
            jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .expect("JWT_EXPIRATION_HOURS must be a number"),
            webhook_secret: std::env::var("WEBHOOK_SECRET").unwrap_or_default(),
        }
    }
}
