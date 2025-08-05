use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::common::error::ApiError;

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum IncidentSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

#[derive(Serialize)]
pub struct SecurityIncident {
    id: Uuid,
    title: String,
    description: String,
    severity: IncidentSeverity,
    status: IncidentStatus,
    reported_by: Uuid,
    assigned_to: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
    resolution_notes: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateIncidentRequest {
    title: String,
    description: String,
    severity: IncidentSeverity,
    assigned_to: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateIncidentStatusRequest {
    status: IncidentStatus,
    resolution_notes: Option<String>,
}

#[derive(Deserialize)]
pub struct IncidentListQuery {
    severity: Option<IncidentSeverity>,
    status: Option<IncidentStatus>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    page: Option<i32>,
    per_page: Option<i32>,
}

#[derive(Serialize)]
pub struct IncidentListResponse {
    incidents: Vec<SecurityIncident>,
    total: i64,
    page: i32,
    per_page: i32,
}

pub async fn list_incidents(
    query: web::Query<IncidentListQuery>,
) -> Result<HttpResponse, ApiError> {
    // TODO: Implement actual incident retrieval from database
    // This is a placeholder that returns an empty response
    let page = query.page.unwrap_or(1).max(1); // Ensure page is at least 1
    let per_page = query.per_page.unwrap_or(20).min(100);

    let response = IncidentListResponse {
        incidents: Vec::new(),
        total: 0,
        page,
        per_page,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_incident(
    req: web::Json<CreateIncidentRequest>,
    claims: web::ReqData<crate::core::auth::jwt::Claims>,
) -> Result<HttpResponse, ApiError> {
    // Validate request
    if req.title.is_empty() {
        return Err(ApiError::bad_request("Title is required"));
    }
    if req.description.is_empty() {
        return Err(ApiError::bad_request("Description is required"));
    }

    // TODO: Implement actual incident creation in database
    // This is a placeholder that returns a mock incident
    let incident = SecurityIncident {
        id: Uuid::new_v4(),
        title: req.title.clone(),
        description: req.description.clone(),
        severity: req.severity,
        status: IncidentStatus::Open,
        reported_by: claims.into_inner().sub,
        assigned_to: req.assigned_to,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        resolved_at: None,
        resolution_notes: None,
    };

    Ok(HttpResponse::Created().json(incident))
}

pub async fn method_not_allowed() -> HttpResponse {
    HttpResponse::MethodNotAllowed().json(serde_json::json!({
        "message": "Method not allowed for this endpoint",
        "error_type": "MethodNotAllowed"
    }))
}

pub async fn get_incident(
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    // Validate UUID format first
    let _id = match Uuid::parse_str(&path.into_inner()) {
        Ok(uuid) => uuid,
        Err(_) => return Err(ApiError::bad_request("Invalid UUID format")),
    };
    
    // TODO: Implement actual incident retrieval from database
    // This is a placeholder that returns a not found error
    Err(ApiError::not_found("Incident not found"))
}

pub async fn update_incident_status(
    path: web::Path<String>,
    _req: web::Json<UpdateIncidentStatusRequest>,
) -> Result<HttpResponse, ApiError> {
    // Validate UUID format first
    let _id = match Uuid::parse_str(&path.into_inner()) {
        Ok(uuid) => uuid,
        Err(_) => return Err(ApiError::bad_request("Invalid UUID format")),
    };
    
    // TODO: Implement actual incident status update in database
    // This is a placeholder that returns a not found error
    Err(ApiError::not_found("Incident not found"))
}