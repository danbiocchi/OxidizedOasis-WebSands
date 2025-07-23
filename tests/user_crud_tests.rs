#[cfg(test)]
mod tests {
    use actix_web::{test, web, App, http::StatusCode};
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;
    use chrono::Utc;

    use oxidizedoasis_websands::core::user::{User, UserRepositoryTrait, MockUserRepositoryTrait, UserError};
    use oxidizedoasis_websands::api::routes::admin::user_management::{
        update_user_role, update_user_status, UpdateRoleRequest, UpdateStatusRequest,
        list_users, get_user, delete_user, update_user_username, UpdateUsernameRequest
    };
    use oxidizedoasis_websands::core::auth::jwt::{Claims, TokenType};
    use oxidizedoasis_websands::api::error_handling::api_error_handler;
    use oxidizedoasis_websands::common::error::ApiErrorType;

    // Helper function to create a mock user
    fn mock_user(id: Uuid, username: &str, role: &str, is_active: bool) -> User {
        User {
            id,
            username: username.to_string(),
            email: Some(format!("{}@example.com", username)),
            password_hash: "hashed_password".to_string(),
            role: role.to_string(),
            is_active,
            is_email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verification_token: None,
            verification_token_expires_at: None,
        }
    }

    #[actix_rt::test]
    async fn test_update_user_role_self_edit_forbidden() {
        let admin_user_id = Uuid::new_v4();
        let mut mock_repo = MockUserRepositoryTrait::new();

        // Mock repo expectations (not strictly needed for this test as it should fail before DB ops)
        mock_repo.expect_update_role().times(0); // Ensure no DB call is made

        let app_state_repo = Arc::new(mock_repo);

        let claims = Claims {
            sub: admin_user_id, // Admin's own ID
            exp: (Utc::now() + chrono::Duration::days(1)).timestamp(), // Changed to i64
            iat: Utc::now().timestamp(), // Added iat
            nbf: Utc::now().timestamp(), // Added nbf
            jti: Uuid::new_v4().to_string(), // Added jti
            role: "admin".to_string(),
            token_type: TokenType::Access, // Added token_type
            aud: "test_aud".to_string(), // Added aud
            iss: "test_iss".to_string(), // Added iss
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state_repo.clone()))
                .app_data(web::Data::new(claims.clone())) // Simulate authenticated admin
                .service(
                    web::resource("/api/admin/users/{id}/role")
                        .route(web::put().to(update_user_role))
                )
                .wrap_fn(api_error_handler)
        ).await;

        let req_payload = UpdateRoleRequest { role: "admin".to_string() };
        let req = test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/role", admin_user_id)) // Attempting to edit self
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], json!(false));
        assert_eq!(body["error_type"], json!(ApiErrorType::Authorization.to_string()));
        assert!(body["message"].as_str().unwrap().contains("You cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_other_user_success() {
        let admin_user_id = Uuid::new_v4();
        let target_user_id = Uuid::new_v4();
        let target_user = mock_user(target_user_id, "target_user", "user", true);
        
        let mut mock_repo = MockUserRepositoryTrait::new();
        let updated_target_user = User { role: "admin".to_string(), ..target_user.clone() };

        mock_repo.expect_update_role()
            .withf(move |id, role| *id == target_user_id && role == "admin")
            .times(1)
            .returning(move |_, _| Ok(Some(updated_target_user.clone())));

        let app_state_repo = Arc::new(mock_repo);

        let claims = Claims {
            sub: admin_user_id, // Admin's ID
            exp: (Utc::now() + chrono::Duration::days(1)).timestamp(), // Changed to i64
            iat: Utc::now().timestamp(), // Added iat
            nbf: Utc::now().timestamp(), // Added nbf
            jti: Uuid::new_v4().to_string(), // Added jti
            role: "admin".to_string(),
            token_type: TokenType::Access, // Added token_type
            aud: "test_aud".to_string(), // Added aud
            iss: "test_iss".to_string(), // Added iss
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state_repo.clone()))
                .app_data(web::Data::new(claims.clone()))
                .service(
                    web::resource("/api/admin/users/{id}/role")
                        .route(web::put().to(update_user_role))
                )
                .wrap_fn(api_error_handler)
        ).await;

        let req_payload = UpdateRoleRequest { role: "admin".to_string() };
        let req = test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/role", target_user_id)) // Editing another user
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], json!(true));
        assert_eq!(body["data"]["role"], json!("admin"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_self_edit_forbidden() {
        let admin_user_id = Uuid::new_v4();
        let mut mock_repo = MockUserRepositoryTrait::new();

        mock_repo.expect_update_status().times(0);

        let app_state_repo = Arc::new(mock_repo);

        let claims = Claims {
            sub: admin_user_id, // Admin's own ID
            exp: (Utc::now() + chrono::Duration::days(1)).timestamp(), // Changed to i64
            iat: Utc::now().timestamp(), // Added iat
            nbf: Utc::now().timestamp(), // Added nbf
            jti: Uuid::new_v4().to_string(), // Added jti
            role: "admin".to_string(),
            token_type: TokenType::Access, // Added token_type
            aud: "test_aud".to_string(), // Added aud
            iss: "test_iss".to_string(), // Added iss
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state_repo.clone()))
                .app_data(web::Data::new(claims.clone()))
                .service(
                    web::resource("/api/admin/users/{id}/status")
                        .route(web::put().to(update_user_status))
                )
                .wrap_fn(api_error_handler)
        ).await;

        let req_payload = UpdateStatusRequest { is_active: false };
        let req = test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/status", admin_user_id)) // Attempting to edit self
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], json!(false));
        assert_eq!(body["error_type"], json!(ApiErrorType::Authorization.to_string()));
        assert!(body["message"].as_str().unwrap().contains("You cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_other_user_success() {
        let admin_user_id = Uuid::new_v4();
        let target_user_id = Uuid::new_v4();
        let target_user = mock_user(target_user_id, "target_user", "user", true);
        
        let mut mock_repo = MockUserRepositoryTrait::new();
        let updated_target_user = User { is_active: false, ..target_user.clone() };

        mock_repo.expect_update_status()
            .withf(move |id, is_active| *id == target_user_id && !*is_active)
            .times(1)
            .returning(move |_, _| Ok(Some(updated_target_user.clone())));

        let app_state_repo = Arc::new(mock_repo);

        let claims = Claims {
            sub: admin_user_id, // Admin's ID
            exp: (Utc::now() + chrono::Duration::days(1)).timestamp(), // Changed to i64
            iat: Utc::now().timestamp(), // Added iat
            nbf: Utc::now().timestamp(), // Added nbf
            jti: Uuid::new_v4().to_string(), // Added jti
            role: "admin".to_string(),
            token_type: TokenType::Access, // Added token_type
            aud: "test_aud".to_string(), // Added aud
            iss: "test_iss".to_string(), // Added iss
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state_repo.clone()))
                .app_data(web::Data::new(claims.clone()))
                .service(
                    web::resource("/api/admin/users/{id}/status")
                        .route(web::put().to(update_user_status))
                )
                .wrap_fn(api_error_handler)
        ).await;

        let req_payload = UpdateStatusRequest { is_active: false };
        let req = test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/status", target_user_id)) // Editing another user
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], json!(true));
        // The UserAdminView for the response doesn't directly include `is_active`.
        // We trust the handler uses the updated user from repo for its response.
        // If UserAdminView is updated to include is_active, this assertion can be more specific.
        // For now, checking success is sufficient as the repo mock ensures correct data was returned.
    }

    // Example test for update_user_username (to show self-edit is also blocked there by current code)
    // This is not strictly required by the subtask but confirms the pattern.
    #[actix_rt::test]
    async fn test_update_user_username_self_edit_forbidden() {
        let admin_user_id = Uuid::new_v4();
        let mut mock_repo = MockUserRepositoryTrait::new();

        mock_repo.expect_find_by_id().times(0); // Should fail before this
        mock_repo.expect_update_username().times(0);

        let app_state_repo = Arc::new(mock_repo);

        let claims = Claims {
            sub: admin_user_id, // Admin's own ID
            exp: (Utc::now() + chrono::Duration::days(1)).timestamp(), // Changed to i64
            iat: Utc::now().timestamp(), // Added iat
            nbf: Utc::now().timestamp(), // Added nbf
            jti: Uuid::new_v4().to_string(), // Added jti
            role: "admin".to_string(),
            token_type: TokenType::Access, // Added token_type
            aud: "test_aud".to_string(), // Added aud
            iss: "test_iss".to_string(), // Added iss
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state_repo.clone()))
                .app_data(web::Data::new(claims.clone())) 
                .service(
                    web::resource("/api/admin/users/{id}/username")
                        .route(web::put().to(update_user_username))
                )
                .wrap_fn(api_error_handler)
        ).await;

        let req_payload = UpdateUsernameRequest { username: "new_admin_name".to_string() };
        let req = test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/username", admin_user_id)) // Attempting to edit self
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], json!(false));
        assert_eq!(body["error_type"], json!(ApiErrorType::Authorization.to_string()));
    }
}
