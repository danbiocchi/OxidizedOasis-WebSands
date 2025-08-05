use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use sqlx::PgPool;
use std::time::Instant;

// To store the application start time
lazy_static::lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime: String,
    database_status: String,
}

async fn health_check(pool: web::Data<PgPool>) -> impl Responder {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let uptime = START_TIME.elapsed().as_secs_f64().to_string() + " seconds";

    let db_status = match pool.acquire().await {
        Ok(mut conn) => {
            match sqlx::query("SELECT 1").execute(&mut *conn).await {
                Ok(_) => "OK".to_string(),
                Err(e) => {
                    log::error!("Database ping failed: {e}");
                    "Error".to_string()
                }
            }
        }
        Err(e) => {
            log::error!("Failed to acquire database connection: {e}");
            "Error".to_string()
        }
    };

    HttpResponse::Ok().json(HealthResponse {
        status: "OK".to_string(),
        version,
        uptime,
        database_status: db_status,
    })
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/health", web::get().to(health_check));
}
