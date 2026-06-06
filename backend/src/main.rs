mod bio_park_tcp;
mod biometric_events;
mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod routes;
mod shift_logic;
mod salary_split;
mod statutory_logic;
mod salary_logic;
mod leave_type_logic;
mod payroll_logic;
mod attendance_logic;
mod workflow_logic;
mod storage;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware as actix_middleware};
use std::sync::Arc;

async fn run_biometric_server(
    host: &str,
    port: u16,
    pool: web::Data<db::DbPool>,
    events: web::Data<biometric_events::BiometricEvents>,
) -> std::io::Result<()> {
    log::info!("📡 Biometric device HTTP listener on http://{}:{}", host, port);
    HttpServer::new(move || {
        App::new()
            .wrap(actix_middleware::Logger::default())
            .app_data(pool.clone())
            .app_data(events.clone())
            // M-CARD / BIO-PARK ADMS endpoints
            .configure(routes::configure_adms)
            // Also support classic iClock endpoints
            .configure(routes::configure_iclock)
    })
    .bind(format!("{host}:{port}"))?
    .run()
    .await
}

async fn run_api_server(
    host: &str,
    port: u16,
    pool: web::Data<db::DbPool>,
    jwt_secret: web::Data<Arc<String>>,
    events: web::Data<biometric_events::BiometricEvents>,
) -> std::io::Result<()> {
    log::info!("🚀 HRM API http://{}:{}", host, port);
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://127.0.0.1:5173")
            .allowed_origin("http://localhost:5174")
            .allowed_origin("http://127.0.0.1:5174")
            .allowed_origin("http://localhost:3000")
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::ACCEPT,
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(actix_middleware::Logger::default())
            .wrap(actix_web::middleware::from_fn(
                crate::middleware::rbac::rbac_middleware,
            ))
            .app_data(pool.clone())
            .app_data(jwt_secret.clone())
            .app_data(events.clone())
            .configure(routes::configure)
    })
    .bind(format!("{host}:{port}"))?
    .run()
    .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let cfg = config::AppConfig::from_env();
    let pool = db::init_pool(&cfg.database_path);
    db::migrations::run_migrations(&pool);

    let jwt_secret = Arc::new(cfg.jwt_secret.clone());
    let pool_data = web::Data::new(pool.clone());
    let jwt_data = web::Data::new(jwt_secret);

    let host = cfg.host.clone();
    let api_port = cfg.port;
    let biometric_port = cfg.biometric_port;
    let tcp_port = cfg.bio_park_tcp_port;

    let events_inner = biometric_events::BiometricEvents::new();
    let events = web::Data::new(events_inner.clone());
    let pool_bio = pool_data.clone();
    let pool_api = pool_data.clone();
    let events_bio = events.clone();
    let events_api = events;

    let host_bio = host.clone();
    let host_api = host.clone();
    let host_tcp = host;
    let pool_tcp = Arc::new(pool);
    let events_tcp = Arc::new(events_inner);

    tokio::spawn(async move {
        if let Err(e) = bio_park_tcp::run(&host_tcp, tcp_port, pool_tcp, events_tcp).await {
            log::error!("BIO-PARK TCP server error: {}", e);
        }
    });

    tokio::try_join!(
        run_api_server(&host_api, api_port, pool_api, jwt_data, events_api),
        run_biometric_server(&host_bio, biometric_port, pool_bio, events_bio),
    )?;

    Ok(())
}
