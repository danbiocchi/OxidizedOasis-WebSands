use actix_web::web;
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::infrastructure::middleware::{admin_validator, CookieAuth, csrf::CsrfProtection};
use log::debug;

pub mod user_management;
pub mod logs;
pub mod security;

pub fn configure(cfg: &mut web::ServiceConfig) {
    debug!("Configuring admin routes");
    let admin_auth = HttpAuthentication::bearer(admin_validator);
    
    cfg.service(
        web::scope("/api/admin")
            .wrap(admin_auth)
            .service(
                web::scope("/users")
                    .route("", web::get().to(user_management::list_users))
                    .route("/{id}", web::get().to(user_management::get_user))
                    .route("/{id}/role", web::put().to(user_management::update_user_role))
                    .route("/{id}/username", web::put().to(user_management::update_user_username))
                    .route("/{id}/status", web::put().to(user_management::update_user_status))
                    .route("/{id}", web::delete().to(user_management::delete_user))
            )
            .service(
                web::scope("/logs")
                    .route("", web::get().to(logs::get_logs))
                    .route("/settings", web::get().to(logs::get_log_settings))
                    .route("/settings", web::put().to(logs::update_log_settings))
            )
            .service(
                web::scope("/security")
                    .route("/incidents", web::get().to(security::list_incidents))
                    .route("/incidents", web::post().to(security::create_incident))
                    .route("/incidents/{id}", web::get().to(security::get_incident))
                    .route("/incidents/{id}/status", web::put().to(security::update_incident_status))
                    .default_service(web::route().to(security::method_not_allowed))
            )
    );
    
    // Configure cookie-based admin routes
    debug!("Configuring cookie-based admin routes");
    
    cfg.service(
        web::scope("/api/cookie/admin")
            .wrap(CookieAuth::new())
            .wrap(CsrfProtection)
            .service(
                web::scope("/users")
                    .route("", web::get().to(user_management::list_users))
                    .route("/{id}", web::get().to(user_management::get_user))
                    .route("/{id}/role", web::put().to(user_management::update_user_role))
                    .route("/{id}/username", web::put().to(user_management::update_user_username))
                    .route("/{id}/status", web::put().to(user_management::update_user_status))
                    .route("/{id}", web::delete().to(user_management::delete_user))
            )
            .service(
                web::scope("/logs")
                    .route("", web::get().to(logs::get_logs))
                    .route("/settings", web::get().to(logs::get_log_settings))
                    .route("/settings", web::put().to(logs::update_log_settings))
            )
            // Add other admin routes as needed
    );
}