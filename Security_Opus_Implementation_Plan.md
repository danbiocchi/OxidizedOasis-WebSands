# Security Implementation Plan - Actionable Technical Guide

## Overall Progress

| Feature Area | Status | % Complete | Notes |
|--------------|--------|------------|-------|
| Database Schema | Not Started | 0% | |
| Event Queue Infrastructure | Not Started | 0% | |
| Event Collection Pipeline | Not Started | 0% | |
| Repository Layer | Not Started | 0% | |
| Correlation Engine | Not Started | 0% | |
| API Implementation | Not Started | 0% | |
| WebSocket Real-time | Not Started | 0% | |
| Frontend Integration | Not Started | 0% | |
| Testing Infrastructure | Not Started | 0% | |

---

## Phase 1: Database Schema Implementation

### Database Migration Setup
- [ ] Create migration file: `migrations/20250810010344_add_security_tables.sql`
- [ ] Add custom enum types
- [ ] Create security_incidents table (base table, no dependencies)
- [ ] Create security_events table with partitioning
- [ ] Create correlation and history tables
- [ ] Add all performance indexes
- [ ] Create maintenance functions
- [ ] Run migration and verify schema

#### Step 1: Create Custom Types
Add to migration file:
```sql
-- Custom Types
CREATE TYPE incident_severity AS ENUM ('low', 'medium', 'high', 'critical');
CREATE TYPE incident_status AS ENUM ('open', 'investigating', 'resolved', 'closed', 'escalated');
CREATE TYPE event_severity AS ENUM ('info', 'low', 'medium', 'high', 'critical');
CREATE TYPE event_outcome AS ENUM ('success', 'failure', 'blocked', 'allowed');
CREATE TYPE correlation_type AS ENUM ('temporal', 'source_based', 'user_based', 'pattern_based');

CREATE TYPE security_event_type AS ENUM (
    'authentication_failure',
    'authentication_success',
    'authorization_failure',
    'password_change',
    'account_lockout',
    'suspicious_login',
    'data_access',
    'data_modification',
    'privilege_escalation',
    'malware_detection',
    'network_intrusion',
    'ddos_attack',
    'sql_injection',
    'xss_attempt',
    'csrf_attempt',
    'file_integrity_violation',
    'configuration_change',
    'system_anomaly',
    'api_abuse',
    'rate_limit_exceeded'
);
```

#### Step 2: Create Base Tables
```sql
-- Security Incidents Table (no foreign key dependencies except users)
CREATE TABLE security_incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    incident_type VARCHAR(50) NOT NULL,
    severity incident_severity NOT NULL,
    status incident_status NOT NULL DEFAULT 'open',
    reported_by UUID REFERENCES users(id),
    assigned_to UUID REFERENCES users(id),
    affected_systems TEXT[],
    event_count INTEGER DEFAULT 0,
    first_event_at TIMESTAMPTZ,
    last_event_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,
    tags TEXT[],
    metadata JSONB DEFAULT '{}'::jsonb,
    
    CONSTRAINT chk_resolution_consistency 
        CHECK ((status IN ('resolved', 'closed') AND resolved_at IS NOT NULL) 
            OR (status NOT IN ('resolved', 'closed') AND resolved_at IS NULL))
);
```

#### Step 3: Create Partitioned Events Table
```sql
-- Security Events Table (Partitioned)
CREATE TABLE security_events (
    id UUID DEFAULT gen_random_uuid(),
    event_type security_event_type NOT NULL,
    event_subtype VARCHAR(50),
    severity event_severity NOT NULL,
    user_id UUID REFERENCES users(id),
    session_id VARCHAR(255),
    source_ip INET,
    user_agent TEXT,
    endpoint VARCHAR(500),
    method VARCHAR(10),
    status_code INTEGER,
    message TEXT NOT NULL,
    details JSONB DEFAULT '{}'::jsonb,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMPTZ,
    incident_id UUID REFERENCES security_incidents(id),
    correlation_id UUID,
    outcome event_outcome,
    processed BOOLEAN DEFAULT false,
    retry_count INTEGER DEFAULT 0,
    
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Create initial partitions
CREATE TABLE security_events_2025_01 PARTITION OF security_events
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
CREATE TABLE security_events_2025_02 PARTITION OF security_events
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');
CREATE TABLE security_events_2025_03 PARTITION OF security_events
    FOR VALUES FROM ('2025-03-01') TO ('2025-04-01');
```

#### Step 4: Add Critical Performance Indexes
```sql
-- Event indexes (most critical for performance)
CREATE INDEX CONCURRENTLY idx_events_timestamp_desc 
    ON security_events (timestamp DESC);
CREATE INDEX CONCURRENTLY idx_events_type_timestamp 
    ON security_events (event_type, timestamp DESC);
CREATE INDEX CONCURRENTLY idx_events_severity_critical 
    ON security_events (severity, timestamp DESC) 
    WHERE severity IN ('high', 'critical');
CREATE INDEX CONCURRENTLY idx_events_unprocessed 
    ON security_events (timestamp) 
    WHERE processed = false;
CREATE INDEX CONCURRENTLY idx_events_user_timestamp 
    ON security_events (user_id, timestamp DESC) 
    WHERE user_id IS NOT NULL;
CREATE INDEX CONCURRENTLY idx_events_ip 
    ON security_events USING gist (source_ip inet_ops);

-- Incident indexes
CREATE INDEX CONCURRENTLY idx_incidents_status 
    ON security_incidents (status, severity, created_at DESC);
CREATE INDEX CONCURRENTLY idx_incidents_open_critical 
    ON security_incidents (severity, created_at DESC) 
    WHERE status = 'open';

-- JSONB indexes
CREATE INDEX CONCURRENTLY idx_events_details_gin 
    ON security_events USING gin(details);
CREATE INDEX CONCURRENTLY idx_incidents_metadata_gin 
    ON security_incidents USING gin(metadata);
```

#### Step 5: Run Migration
```bash
# Run migration
psql -d oasis -f migrations/20250810010344_add_security_tables.sql

# Verify tables created
psql -d oasis -c "\dt security_*"

# Verify indexes created
psql -d oasis -c "\di idx_*"
```

---

## Phase 2: Event Queue Infrastructure

### Persistent Queue Implementation
- [ ] Create directory structure: `src/core/security/`
- [ ] Create `src/core/security/mod.rs`
- [ ] Implement `src/core/security/event_queue.rs`
- [ ] Implement `src/core/security/queue_processor.rs`
- [ ] Add dependencies to `Cargo.toml`
- [ ] Create queue directory: `queue/`
- [ ] Write unit tests for queue
- [ ] Test queue persistence and recovery

#### Step 1: Add Dependencies to Cargo.toml
```toml
[dependencies]
# Add to existing dependencies
tokio = { version = "1.32", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.4", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
tracing = "0.1"
```

#### Step 2: Create Module Structure
```rust
// src/core/security/mod.rs
pub mod event_queue;
pub mod queue_processor;
pub mod event_collector;
pub mod types;

pub use event_queue::PersistentEventQueue;
pub use queue_processor::QueueProcessor;
pub use event_collector::EventCollector;
pub use types::*;
```

#### Step 3: Implement Types
Create `src/core/security/types.rs`:
```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    AuthenticationFailure,
    AuthenticationSuccess,
    AuthorizationFailure,
    PasswordChange,
    AccountLockout,
    SuspiciousLogin,
    DataAccess,
    DataModification,
    PrivilegeEscalation,
    SqlInjection,
    XssAttempt,
    ApiAbuse,
    RateLimitExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventOutcome {
    Success,
    Failure,
    Blocked,
    Allowed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: Uuid,
    pub event_type: SecurityEventType,
    pub severity: EventSeverity,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub source_ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<u16>,
    pub message: String,
    pub details: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub incident_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub outcome: Option<EventOutcome>,
    pub processed: bool,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSecurityEvent {
    pub event_type: SecurityEventType,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub source_ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<u16>,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: Option<DateTime<Utc>>,
}
```

#### Step 4: Implement Queue Core (Complete Implementation)
Create the full `src/core/security/event_queue.rs` file with all the code from the original plan.

#### Step 5: Implement Queue Processor
Create the full `src/core/security/queue_processor.rs` file with all the code from the original plan.

#### Step 6: Create Queue Tests
Create `tests/queue_tests.rs` with the complete test suite from the original plan.

#### Step 7: Run Queue Tests
```bash
# Create queue directory
mkdir -p queue

# Run tests
cargo test event_queue -- --nocapture
cargo test test_queue_survives_restart -- --nocapture
cargo test test_queue_handles_db_outage -- --nocapture
cargo test test_queue_memory_bounds -- --nocapture
```

---

## Phase 3: Event Collection Pipeline

### Event Collector Implementation
- [ ] Create `src/core/security/event_collector.rs`
- [ ] Implement event validation
- [ ] Implement event enrichment
- [ ] Create circuit breaker
- [ ] Create security middleware
- [ ] Create event generator binary
- [ ] Test event collection rate

#### Step 1: Implement Event Collector
Create complete `src/core/security/event_collector.rs` with EventCollector, EventEnricher, EventValidator, and CircuitBreaker implementations.

#### Step 2: Create Security Middleware
Create `src/infrastructure/middleware/security_logging.rs`:
```rust
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

pub struct SecurityLoggingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for SecurityLoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityLoggingService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityLoggingService { service }))
    }
}

pub struct SecurityLoggingService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for SecurityLoggingService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract security event details
        let event = extract_security_event(&req);
        
        // Log event asynchronously
        tokio::spawn(async move {
            if let Some(collector) = GLOBAL_EVENT_COLLECTOR.get() {
                let _ = collector.collect_event(event).await;
            }
        });

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
```

#### Step 3: Register Middleware in main.rs
```rust
// In src/main.rs, add to app configuration
app.wrap(SecurityLoggingMiddleware)
```

#### Step 4: Create Event Generator Binary
Create `src/bin/event_generator.rs` with the complete implementation from the original plan.

#### Step 5: Test Event Generation
```bash
# Build the generator
cargo build --bin event_generator

# Test at 100 events/second for 60 seconds
cargo run --bin event_generator -- --rate 100 --duration-secs 60

# Test with attack pattern
cargo run --bin event_generator -- --rate 100 --duration-secs 60 --attack-pattern brute_force
```

---

## Phase 4: Repository Layer Implementation

### Security Event Repository
- [ ] Create `src/core/security/repositories/mod.rs`
- [ ] Implement `security_event_repository.rs`
- [ ] Implement `security_incident_repository.rs`
- [ ] Add bulk insert operations
- [ ] Implement prepared statements
- [ ] Configure connection pooling
- [ ] Add repository tests

#### Step 1: Create Repository Module
```rust
// src/core/security/repositories/mod.rs
pub mod security_event_repository;
pub mod security_incident_repository;

pub use security_event_repository::SecurityEventRepository;
pub use security_incident_repository::SecurityIncidentRepository;
```

#### Step 2: Implement Event Repository
Create `src/core/security/repositories/security_event_repository.rs`:
```rust
use sqlx::{PgPool, postgres::PgQueryResult};
use std::sync::Arc;
use uuid::Uuid;
use anyhow::Result;

pub struct SecurityEventRepository {
    db_pool: Arc<PgPool>,
}

impl SecurityEventRepository {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub async fn bulk_insert_events(&self, events: Vec<SecurityEvent>) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let mut tx = self.db_pool.begin().await?;
        
        for event in events {
            sqlx::query!(
                r#"
                INSERT INTO security_events (
                    id, event_type, severity, user_id, session_id,
                    source_ip, user_agent, endpoint, method, status_code,
                    message, details, timestamp, outcome, processed
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                "#,
                event.id,
                event.event_type as _,
                event.severity as _,
                event.user_id,
                event.session_id,
                event.source_ip.map(|ip| ip.to_string()),
                event.user_agent,
                event.endpoint,
                event.method,
                event.status_code.map(|c| c as i32),
                event.message,
                event.details,
                event.timestamp,
                event.outcome as _,
                event.processed
            )
            .execute(&mut tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_unprocessed_events(&self, limit: i64) -> Result<Vec<SecurityEvent>> {
        let events = sqlx::query_as!(
            SecurityEvent,
            r#"
            SELECT * FROM security_events
            WHERE processed = false
            ORDER BY timestamp
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&*self.db_pool)
        .await?;
        
        Ok(events)
    }

    pub async fn mark_events_processed(&self, event_ids: Vec<Uuid>) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE security_events
            SET processed = true, processed_at = NOW()
            WHERE id = ANY($1)
            "#,
            &event_ids
        )
        .execute(&*self.db_pool)
        .await?;
        
        Ok(())
    }
}
```

#### Step 3: Configure Connection Pool
Update `src/infrastructure/database/connection.rs`:
```rust
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(50)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(3600))
        .connect(database_url)
        .await
}
```

---

## Phase 5: Correlation Engine

### Event Correlation Implementation
- [ ] Create `src/core/security/correlation/mod.rs`
- [ ] Implement temporal correlation
- [ ] Implement pattern correlation
- [ ] Create correlation service
- [ ] Add correlation tests
- [ ] Test correlation performance

#### Step 1: Create Correlation Module
```rust
// src/core/security/correlation/mod.rs
pub mod temporal;
pub mod pattern;
pub mod service;

pub use temporal::TemporalCorrelator;
pub use pattern::PatternCorrelator;
pub use service::CorrelationService;
```

#### Step 2: Implement Temporal Correlation
Create `src/core/security/correlation/temporal.rs` with complete implementation from original plan.

#### Step 3: Implement Pattern Correlation
Create `src/core/security/correlation/pattern.rs` with complete implementation from original plan.

#### Step 4: Create Correlation Service
```rust
// src/core/security/correlation/service.rs
use std::sync::Arc;
use crate::core::security::types::*;

pub struct CorrelationService {
    temporal: Arc<TemporalCorrelator>,
    pattern: Arc<PatternCorrelator>,
    db_pool: Arc<PgPool>,
}

impl CorrelationService {
    pub async fn correlate_event(&self, event: &SecurityEvent) -> Result<Vec<EventCorrelation>> {
        let mut correlations = Vec::new();
        
        // Run temporal correlation
        let temporal_results = self.temporal.find_temporal_correlations(event).await?;
        correlations.extend(temporal_results);
        
        // Run pattern correlation
        let pattern_results = self.pattern.find_pattern_correlations(event).await?;
        correlations.extend(pattern_results);
        
        // Store correlations
        for correlation in &correlations {
            self.store_correlation(correlation).await?;
        }
        
        Ok(correlations)
    }
}
```

---

## Phase 6: API Implementation

### Security API Endpoints
- [ ] Update `src/api/routes/admin/security.rs`
- [ ] Implement GET /api/admin/security/incidents
- [ ] Implement POST /api/admin/security/incidents
- [ ] Implement GET /api/admin/security/events
- [ ] Implement pagination
- [ ] Add filtering support
- [ ] Test API endpoints

#### Step 1: Implement Security Incidents API
Update `src/api/routes/admin/security.rs`:
```rust
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct IncidentFilters {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Serialize)]
pub struct IncidentListResponse {
    pub incidents: Vec<SecurityIncident>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

pub async fn get_incidents(
    filters: web::Query<IncidentFilters>,
    incident_service: web::Data<IncidentService>,
) -> Result<HttpResponse, ApiError> {
    let page = filters.page.unwrap_or(1);
    let page_size = filters.page_size.unwrap_or(20);
    
    let (incidents, total) = incident_service
        .get_incidents_paginated(&filters.into_inner(), page, page_size)
        .await?;
    
    Ok(HttpResponse::Ok().json(IncidentListResponse {
        incidents,
        total,
        page,
        page_size,
    }))
}

pub async fn create_incident(
    req: web::Json<CreateIncidentRequest>,
    incident_service: web::Data<IncidentService>,
    claims: JwtClaims,
) -> Result<HttpResponse, ApiError> {
    let mut request = req.into_inner();
    request.created_by = claims.user_id;
    
    let incident = incident_service.create_incident(request).await?;
    
    Ok(HttpResponse::Created().json(incident))
}

pub async fn get_events(
    filters: web::Query<EventFilters>,
    event_service: web::Data<EventService>,
) -> Result<HttpResponse, ApiError> {
    let page = filters.page.unwrap_or(1);
    let page_size = filters.page_size.unwrap_or(50);
    
    let (events, total) = event_service
        .get_events_paginated(&filters.into_inner(), page, page_size)
        .await?;
    
    Ok(HttpResponse::Ok().json(EventListResponse {
        events,
        total,
        page,
        page_size,
    }))
}
```

#### Step 2: Register Routes
```rust
// In src/api/routes/admin/mod.rs
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .wrap(AdminMiddleware)
            .service(
                web::scope("/security")
                    .route("/incidents", web::get().to(security::get_incidents))
                    .route("/incidents", web::post().to(security::create_incident))
                    .route("/incidents/{id}", web::get().to(security::get_incident))
                    .route("/incidents/{id}", web::put().to(security::update_incident))
                    .route("/events", web::get().to(security::get_events))
                    .route("/events/stats", web::get().to(security::get_event_stats))
            )
    );
}
```

---

## Phase 7: WebSocket Implementation

### Real-time Updates
- [ ] Create `src/core/security/websocket/mod.rs`
- [ ] Implement WebSocket actor
- [ ] Add authentication
- [ ] Implement subscription management
- [ ] Add rate limiting
- [ ] Test concurrent connections

#### Step 1: Add WebSocket Dependencies
```toml
[dependencies]
actix-web-actors = "4.2"
actix = "0.13"
```

#### Step 2: Implement WebSocket Actor
Create `src/core/security/websocket/actor.rs`:
```rust
use actix::{Actor, StreamHandler, AsyncContext, Handler};
use actix_web_actors::ws;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct SecurityWebSocket {
    user_id: Option<Uuid>,
    user_role: Option<UserRole>,
    subscriptions: HashSet<String>,
    hb: Instant,
}

impl Actor for SecurityWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

impl SecurityWebSocket {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SecurityWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                if let Ok(command) = serde_json::from_str::<WebSocketCommand>(&text) {
                    self.handle_command(command, ctx);
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}
```

#### Step 3: Create WebSocket Route
```rust
// In src/api/routes/mod.rs
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

pub async fn websocket_route(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    // Validate JWT token from query params or headers
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));
    
    let user_info = validate_token(token)?;
    
    ws::start(
        SecurityWebSocket::new(user_info.user_id, user_info.role),
        &req,
        stream,
    )
}

// Register route
cfg.route("/ws", web::get().to(websocket_route));
```

---

## Phase 8: Frontend Integration

### Dashboard Connection
- [ ] Update `frontend/src/services/security.rs`
- [ ] Create WebSocket service
- [ ] Update incident list component
- [ ] Add real-time updates
- [ ] Remove mock data
- [ ] Test dashboard functionality

#### Step 1: Create Security Service
Create `frontend/src/services/security.rs`:
```rust
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncident {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub severity: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn fetch_incidents(page: u32, filters: &IncidentFilters) -> Result<IncidentListResponse, String> {
    let response = Request::get(&format!("/api/admin/security/incidents?page={}", page))
        .header("Authorization", &get_auth_token())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.ok() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err(format!("Failed to fetch incidents: {}", response.status()))
    }
}

pub async fn create_incident(incident: CreateIncidentRequest) -> Result<SecurityIncident, String> {
    let response = Request::post("/api/admin/security/incidents")
        .header("Authorization", &get_auth_token())
        .json(&incident)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.ok() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err(format!("Failed to create incident: {}", response.status()))
    }
}
```

#### Step 2: Create WebSocket Service
Create `frontend/src/services/websocket.rs`:
```rust
use web_sys::{MessageEvent, WebSocket};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub struct SecurityWebSocket {
    ws: WebSocket,
    on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl SecurityWebSocket {
    pub fn new(url: &str, on_update: impl Fn(SecurityUpdate) + 'static) -> Result<Self, JsValue> {
        let ws = WebSocket::new(url)?;
        
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                if let Ok(update) = serde_json::from_str::<SecurityUpdate>(&text.as_string().unwrap()) {
                    on_update(update);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        
        Ok(Self