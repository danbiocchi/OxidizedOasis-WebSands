use std::sync::Arc;

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::core::user::{User, UserRepositoryTrait};
use crate::common::error::{ApiError, ApiErrorType};
use crate::api::responses::ApiResponse;
use serde_json::json;
use crate::core::auth::jwt::Claims;

use log::debug;

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    role: String,
}

#[derive(Deserialize)]
pub struct UpdateUsernameRequest {
    username: String,
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    is_active: bool,
}

#[derive(Serialize)]
pub struct UserListResponse {
    users: Vec<UserAdminView>,
}

#[derive(Serialize, Debug)]
pub struct UserAdminView {
    id: Uuid,
    username: String,
    email: Option<String>,
    role: String,
    is_email_verified: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    is_active: bool, // Added is_active field
}

impl From<User> for UserAdminView {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            is_active: user.is_active, // Map is_active from User
            role: user.role,
            is_email_verified: user.is_email_verified,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

pub async fn list_users(
    repo: web::Data<std::sync::Arc<dyn UserRepositoryTrait>>,
) -> Result<HttpResponse, ApiError> {
    debug!("Handling GET /api/admin/users -> list_users");
    let users = repo.find_all()
        .await
        .map_err(|e| {
            debug!("Error fetching users: {:?}", e);
            ApiError::from(e)
        })?;

    debug!("Fetched {} users from repository", users.len());
    let users_view: Vec<UserAdminView> = users.into_iter().map(UserAdminView::from).collect();
    debug!("Prepared {} user views", users_view.len());

    Ok(HttpResponse::Ok().json(ApiResponse::success(UserListResponse {
        users: users_view
    })))
}

pub async fn get_user(
    id: web::Path<Uuid>,
    repo: web::Data<Arc<dyn UserRepositoryTrait>>,
) -> Result<HttpResponse, ApiError> {
    debug!("Handling GET /api/admin/users/{} -> get_user", id);
    let user = repo.find_by_id(*id)
        .await
        .map_err(|e| {
            debug!("Error fetching user {}: {:?}", id, e);
            ApiError::from(e)
        })?
        .ok_or_else(|| {
            debug!("User {} not found", id);
            ApiError::not_found("User not found")
        })?;

    let user_view = UserAdminView::from(user);
    debug!("Found user: {:?}", &user_view);

    Ok(HttpResponse::Ok().json(ApiResponse::success(user_view)))
}

pub async fn update_user_role(
    id: web::Path<Uuid>,
    req: web::Json<UpdateRoleRequest>,
    repo: web::Data<Arc<dyn UserRepositoryTrait>>,
    claims: Option<web::ReqData<Claims>>,
) -> Result<HttpResponse, ApiError> {
    debug!("Handling PUT /api/admin/users/{}/role -> update_user_role with role: {}",
           id, req.role);
    
    // Check if user is trying to edit their own account
    if let Some(claims) = claims {
        // The sub field in Claims is already a Uuid
        if claims.sub == *id {
            debug!("User {} attempted to edit their own role", id);
            return Err(ApiError::new(
                "You cannot edit your own account. This could lead to session inconsistency issues.",
                ApiErrorType::Authorization
            ));
        }
    }
    
    // Validate role
    if !["user", "admin"].contains(&req.role.as_str()) {
        debug!("Invalid role attempted: {}", req.role);
        return Err(ApiError::bad_request("Invalid role"));
    }

    let user = repo.update_role(*id, &req.role)
        .await
        .map_err(|e| {
            debug!("Error updating role for user {}: {:?}", id, e);
            ApiError::from(e)
        })?
        .ok_or_else(|| {
            debug!("User {} not found for role update", id);
            ApiError::not_found("User not found")
        })?;

    let user_view = UserAdminView::from(user);
    debug!("Updated user role successfully: {:?}", &user_view);

    Ok(HttpResponse::Ok().json(ApiResponse::success(user_view)))
}

pub async fn update_user_username(
    id: web::Path<Uuid>,
    req: web::Json<UpdateUsernameRequest>,
    repo: web::Data<Arc<dyn UserRepositoryTrait>>,
    claims: Option<web::ReqData<Claims>>,
) -> Result<HttpResponse, ApiError> {
    debug!("Handling PUT /api/admin/users/{}/username -> update_user_username with username: {}",
           id, req.username);
    
    // Check if user is trying to edit their own account
    if let Some(claims) = claims {
        // The sub field in Claims is already a Uuid
        if claims.sub == *id {
            debug!("User {} attempted to edit their own username", id);
            return Err(ApiError::new(
                "You cannot edit your own account. This could lead to session inconsistency issues.",
                ApiErrorType::Authorization
            ));
        }
    }
    
    // Validate username - ensure it's not empty
    if req.username.trim().is_empty() {
        debug!("Empty username attempted");
        return Err(ApiError::bad_request("Username cannot be empty"));
    }
    
    // Find the user first to check if it exists
    let _user = repo.find_by_id(*id)
        .await
        .map_err(|e| {
            debug!("Error finding user {}: {:?}", id, e);
            ApiError::from(e)
        })?
        .ok_or_else(|| {
            debug!("User {} not found for username update", id);
            ApiError::not_found("User not found")
        })?;
    
    // Update the username
    let updated_user = repo.update_username(*id, &req.username)
        .await
        .map_err(|e| {
            debug!("Error updating username for user {}: {:?}", id, e);
            ApiError::from(e)
        })?
        .ok_or_else(|| {
            debug!("User {} not found after username update", id);
            ApiError::not_found("User not found")
        })?;
    
    let user_view = UserAdminView::from(updated_user);
    debug!("Updated user username successfully: {:?}", &user_view);
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(user_view)))
}

pub async fn update_user_status(
    id: web::Path<Uuid>,
    req: web::Json<UpdateStatusRequest>,
    repo: web::Data<Arc<dyn UserRepositoryTrait>>,
    claims: Option<web::ReqData<Claims>>,
) -> Result<HttpResponse, ApiError> {
    debug!("Handling PUT /api/admin/users/{}/status -> update_user_status with is_active: {}",
           id, req.is_active);

    // Check if user is trying to edit their own account
    if let Some(claims) = claims {
        // The sub field in Claims is already a Uuid
        if claims.sub == *id {
            debug!("User {} attempted to edit their own status", id);
            return Err(ApiError::new(
                "You cannot edit your own account. This could lead to session inconsistency issues.",
                ApiErrorType::Authorization
            ));
        }
    }

    let user = repo.update_status(*id, req.is_active)
        .await
        .map_err(|e| {
            debug!("Error updating status for user {}: {:?}", id, e);
            ApiError::from(e)
        })?
        .ok_or_else(|| {
            debug!("User {} not found for status update", id);
            ApiError::not_found("User not found")
        })?;

    let user_view = UserAdminView::from(user);
    debug!("Updated user status successfully: {:?}", &user_view);

    Ok(HttpResponse::Ok().json(ApiResponse::success(user_view)))
}

pub async fn delete_user(
    id: web::Path<Uuid>,
    repo: web::Data<Arc<dyn UserRepositoryTrait>>,
    claims: Option<web::ReqData<Claims>>,
) -> Result<HttpResponse, ApiError> {
    debug!("Handling DELETE /api/admin/users/{} -> delete_user", id);

    // Check if user is trying to delete their own account
    if let Some(claims) = claims {
        // The sub field in Claims is already a Uuid
        if claims.sub == *id {
            debug!("User {} attempted to delete their own account", id);
            return Err(ApiError::new(
                "You cannot delete your own account. This could lead to session inconsistency issues.",
                ApiErrorType::Authorization
            ));
        }
    }

    let deleted = repo.delete(*id)
        .await
        .map_err(|e| {
            debug!("Error deleting user {}: {:?}", id, e);
            ApiError::from(e)
        })?;

    // Check if a user was found and deleted
    if !deleted {
        debug!("User {} not found for deletion", id);
        return Err(ApiError::not_found("User not found"));
    }

    debug!("Deleted user {} successfully", id);

    // Return success response with a message
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "message": "User deleted successfully"
    })))
}
