# Security Implementation Plan - Enhanced Version
*Comprehensive Security Event Monitoring and Incident Management System*

## Table of Contents
1. [Executive Summary](#executive-summary)
2. [Current System Analysis](#current-system-analysis)
3. [Architecture Overview](#architecture-overview)
4. [Database Schema Design](#database-schema-design)
5. [Core Components](#core-components)
6. [Real-time Processing Architecture](#real-time-processing-architecture)
7. [Event Correlation Engine](#event-correlation-engine)
8. [Security Considerations](#security-considerations)
9. [Performance Optimization](#performance-optimization)
10. [Implementation Timeline](#implementation-timeline)
11. [Testing Strategy](#testing-strategy)
12. [Success Criteria](#success-criteria)
13. [Risk Assessment](#risk-assessment)
14. [Deployment Strategy](#deployment-strategy)

---

## Executive Summary

This document outlines the comprehensive implementation plan for transforming the existing placeholder security incidents system into a fully functional security event monitoring and incident management platform. The system will provide real-time security event collection, intelligent correlation, automated incident creation, and comprehensive management workflows.

### Key Objectives
- **Real-time Event Processing**: Handle 10,000+ security events per minute
- **Intelligent Correlation**: Automated incident creation from related events
- **Comprehensive Management**: Full incident lifecycle management
- **Performance**: Sub-second response times for dashboard queries
- **Security**: Zero-trust architecture with comprehensive audit trails

---

## Current System Analysis

### Existing Infrastructure
The current system provides a solid foundation with:

#### Authentication System
- **JWT Implementation**: Comprehensive token management (`src/core/auth/jwt.rs`)
  - Access/refresh token pairs
  - Token revocation with active tracking
  - Secure token validation and claims extraction
- **User Management**: Complete user lifecycle (`src/core/auth/service.rs`)
  - Registration with email verification
  - Password reset with secure tokens
  - Role-based access control

#### Database Schema
- **Users Table**: Complete user management with roles
- **Password Resets**: Secure token-based password recovery
- **Revoked Tokens**: JWT blacklist implementation
- **Active Tokens**: Session tracking and management

#### Frontend Components
- **Security Incidents UI**: Complete implementation (`frontend/src/pages/dashboard/admin/security_incidents.rs`)
  - Real-time dashboard with filtering
  - Modal detail views
  - Status management interface
- **Authentication Flow**: Full login/register/reset workflow

#### Backend API Structure
- **Admin Routes**: Proper structure with authorization middleware
- **Security Endpoints**: Framework ready for implementation
- **Middleware Stack**: Authentication and admin authorization

### Current Limitations
1. **Placeholder Implementations**: API endpoints return empty responses
2. **No Event Collection**: Missing security event ingestion system
3. **No Real-time Processing**: No WebSocket or streaming infrastructure
4. **Limited Database Schema**: Missing security-specific tables

---

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend Layer                          │
├─────────────────────────────────────────────────────────────────┤
│  • Yew-based SPA with WebAssembly                             │
│  • Real-time WebSocket connections                             │
│  • Role-based UI components                                    │
│  • Responsive dashboard with filtering                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      API Gateway Layer                          │
├─────────────────────────────────────────────────────────────────┤
│  • Actix Web HTTP server                                       │
│  • JWT authentication middleware                               │
│  • Rate limiting and CORS                                      │
│  • Request/response logging                                    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Service Layer                             │
├─────────────────────────────────────────────────────────────────┤
│  • Event Collection Service                                    │
│  • Incident Management Service                                 │
│  • Real-time Notification Service                              │
│  • Event Correlation Engine                                    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Processing Layer                           │
├─────────────────────────────────────────────────────────────────┤
│  • Event Stream Processor (Tokio-based)                        │
│  • Rule Engine for Correlation                                 │
│  • Alerting and Notification System                            │
│  • Background Task Scheduler                                   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Data Layer                                │
├─────────────────────────────────────────────────────────────────┤
│  • PostgreSQL Database with optimized indexes                  │
│  • Redis for caching and session management                    │
│  • File system for large payloads (optional)                  │
└─────────────────────────────────────────────────────────────────┘
```

### Technology Stack
- **Backend**: Rust with Actix Web framework
- **Frontend**: Yew (WebAssembly)
- **Database**: PostgreSQL 14+
- **Caching**: Redis 6+
- **Real-time**: WebSocket with Actix-Web-Actors
- **Authentication**: JWT with RS256 signing

---

## Database Schema Design

### Schema Order and Dependencies
**CRITICAL FIX**: Proper table ordering to resolve foreign key constraints.

```sql
-- Migration: 20250809_security_events_schema.sql

-- Step 1: Create base tables (no dependencies)
CREATE TABLE security_incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    severity incident_severity NOT NULL,
    status incident_status NOT NULL DEFAULT 'open',
    assigned_to UUID REFERENCES users(id),
    created_by UUID REFERENCES users(id) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,
    tags TEXT[],
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Step 2: Create dependent tables
CREATE TABLE security_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type security_event_type NOT NULL,
    severity event_severity NOT NULL,
    source_ip INET,
    user_id UUID REFERENCES users(id),
    incident_id UUID REFERENCES security_incidents(id), -- Fixed: Now references existing table
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),
    action VARCHAR(100),
    outcome event_outcome,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    user_agent TEXT,
    session_id VARCHAR(255),
    details JSONB DEFAULT '{}'::jsonb,
    raw_log TEXT,
    correlation_id UUID,
    processed BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Step 3: Create relationship tables
CREATE TABLE incident_events (
    incident_id UUID REFERENCES security_incidents(id) ON DELETE CASCADE,
    event_id UUID REFERENCES security_events(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    added_by UUID REFERENCES users(id) NOT NULL,
    PRIMARY KEY (incident_id, event_id)
);

-- Step 4: Create audit and tracking tables
CREATE TABLE incident_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id UUID REFERENCES security_incidents(id) ON DELETE CASCADE,
    field_name VARCHAR(100) NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by UUID REFERENCES users(id) NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    change_reason TEXT
);

CREATE TABLE event_correlations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    primary_event_id UUID REFERENCES security_events(id) ON DELETE CASCADE,
    related_event_id UUID REFERENCES security_events(id) ON DELETE CASCADE,
    correlation_type correlation_type NOT NULL,
    confidence_score DECIMAL(3,2) CHECK (confidence_score BETWEEN 0.0 AND 1.0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(primary_event_id, related_event_id, correlation_type)
);
```

### Custom Types
```sql
-- Enums for type safety
CREATE TYPE incident_severity AS ENUM ('low', 'medium', 'high', 'critical');
CREATE TYPE incident_status AS ENUM ('open', 'investigating', 'resolved', 'closed', 'escalated');
CREATE TYPE security_event_type AS ENUM (
    'authentication_failure', 'authentication_success', 'authorization_failure',
    'password_change', 'account_lockout', 'suspicious_login', 'data_access',
    'data_modification', 'privilege_escalation', 'malware_detection',
    'network_intrusion', 'ddos_attack', 'sql_injection', 'xss_attempt',
    'file_integrity_violation', 'configuration_change', 'system_anomaly'
);
CREATE TYPE event_severity AS ENUM ('info', 'low', 'medium', 'high', 'critical');
CREATE TYPE event_outcome AS ENUM ('success', 'failure', 'blocked', 'allowed');
CREATE TYPE correlation_type AS ENUM ('temporal', 'source_based', 'user_based', 'pattern_based');
```

### Performance Indexes
**CRITICAL FIX**: Comprehensive indexing strategy for optimal query performance.

```sql
-- Primary performance indexes
CREATE INDEX CONCURRENTLY idx_security_events_timestamp ON security_events(timestamp DESC);
CREATE INDEX CONCURRENTLY idx_security_events_type_timestamp ON security_events(event_type, timestamp DESC);
CREATE INDEX CONCURRENTLY idx_security_events_severity ON security_events(severity);
CREATE INDEX CONCURRENTLY idx_security_events_user_id ON security_events(user_id);
CREATE INDEX CONCURRENTLY idx_security_events_source_ip ON security_events(source_ip);
CREATE INDEX CONCURRENTLY idx_security_events_incident_id ON security_events(incident_id);
CREATE INDEX CONCURRENTLY idx_security_events_correlation_id ON security_events(correlation_id);
CREATE INDEX CONCURRENTLY idx_security_events_processed ON security_events(processed, timestamp);

-- Incident indexes
CREATE INDEX CONCURRENTLY idx_security_incidents_status ON security_incidents(status);
CREATE INDEX CONCURRENTLY idx_security_incidents_severity ON security_incidents(severity);
CREATE INDEX CONCURRENTLY idx_security_incidents_created_at ON security_incidents(created_at DESC);
CREATE INDEX CONCURRENTLY idx_security_incidents_assigned_to ON security_incidents(assigned_to);

-- Composite indexes for common queries
CREATE INDEX CONCURRENTLY idx_events_user_timestamp ON security_events(user_id, timestamp DESC);
CREATE INDEX CONCURRENTLY idx_events_ip_timestamp ON security_events(source_ip, timestamp DESC);
CREATE INDEX CONCURRENTLY idx_incidents_status_severity ON security_incidents(status, severity);

-- JSONB indexes for metadata queries
CREATE INDEX CONCURRENTLY idx_security_events_details_gin ON security_events USING gin(details);
CREATE INDEX CONCURRENTLY idx_security_incidents_metadata_gin ON security_incidents USING gin(metadata);

-- Full-text search indexes
CREATE INDEX CONCURRENTLY idx_security_incidents_search ON security_incidents USING gin(to_tsvector('english', title || ' ' || COALESCE(description, '')));
```

### Data Retention Strategy
```sql
-- Partitioning for large event tables
CREATE TABLE security_events_y2025m01 PARTITION OF security_events
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

-- Automated cleanup function
CREATE OR REPLACE FUNCTION cleanup_old_security_events()
RETURNS void AS $$
BEGIN
    -- Archive events older than 2 years
    DELETE FROM security_events 
    WHERE timestamp < now() - interval '2 years';
    
    -- Archive resolved incidents older than 1 year
    DELETE FROM security_incidents 
    WHERE status IN ('resolved', 'closed') 
    AND resolved_at < now() - interval '1 year';
END;
$$ LANGUAGE plpgsql;

-- Schedule cleanup (requires pg_cron extension)
SELECT cron.schedule('cleanup-security-events', '0 2 * * 0', 'SELECT cleanup_old_security_events();');
```

---

## Core Components

### 1. Event Collection Service

#### Architecture
```rust
// src/core/security/event_collector.rs
#[derive(Clone)]
pub struct EventCollector {
    db_pool: Arc<PgPool>,
    event_sender: UnboundedSender<SecurityEvent>,
    buffer: Arc<Mutex<VecDeque<SecurityEvent>>>,
    buffer_size: usize,
    flush_interval: Duration,
}

impl EventCollector {
    pub async fn collect_event(&self, event: SecurityEvent) -> Result<(), SecurityError> {
        // Immediate processing for critical events
        if event.severity == EventSeverity::Critical {
            return self.process_immediately(event).await;
        }
        
        // Buffer non-critical events
        self.buffer_event(event).await
    }
    
    async fn buffer_event(&self, event: SecurityEvent) -> Result<(), SecurityError> {
        let mut buffer = self.buffer.lock().await;
        buffer.push_back(event);
        
        if buffer.len() >= self.buffer_size {
            self.flush_buffer().await?;
        }
        
        Ok(())
    }
    
    async fn flush_buffer(&self) -> Result<(), SecurityError> {
        let mut buffer = self.buffer.lock().await;
        let events: Vec<_> = buffer.drain(..).collect();
        drop(buffer);
        
        if !events.is_empty() {
            self.bulk_insert_events(events).await?;
        }
        
        Ok(())
    }
}
```

#### Middleware Integration
```rust
// src/infrastructure/middleware/security_logging.rs
pub struct SecurityLoggingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for SecurityLoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SecurityLoggingService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityLoggingService { service }))
    }
}

impl SecurityLoggingService {
    async fn log_security_event(&self, req: &ServiceRequest) -> Result<(), SecurityError> {
        let event = SecurityEvent {
            event_type: self.determine_event_type(req),
            severity: self.calculate_severity(req),
            source_ip: self.extract_client_ip(req),
            user_id: self.extract_user_id(req),
            timestamp: Utc::now(),
            details: self.capture_request_details(req),
            // ... other fields
        };
        
        GLOBAL_EVENT_COLLECTOR.collect_event(event).await
    }
}
```

### 2. Incident Management Service

#### Core Service Implementation
```rust
// src/core/security/incident_service.rs
#[derive(Clone)]
pub struct IncidentService {
    db_pool: Arc<PgPool>,
    notification_service: Arc<NotificationService>,
    event_correlator: Arc<EventCorrelator>,
}

impl IncidentService {
    pub async fn create_incident(&self, request: CreateIncidentRequest) -> Result<Incident, SecurityError> {
        let incident = Incident {
            id: Uuid::new_v4(),
            title: request.title,
            description: request.description,
            severity: request.severity,
            status: IncidentStatus::Open,
            created_by: request.created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: request.tags.unwrap_or_default(),
            metadata: json!({}),
            ..Default::default()
        };
        
        let saved_incident = self.save_incident(&incident).await?;
        
        // Link related events if provided
        if let Some(event_ids) = request.event_ids {
            self.link_events_to_incident(&saved_incident.id, &event_ids, request.created_by).await?;
        }
        
        // Send notifications
        self.notification_service.incident_created(&saved_incident).await?;
        
        Ok(saved_incident)
    }
    
    pub async fn auto_create_from_correlation(&self, correlation: EventCorrelation) -> Result<Option<Incident>, SecurityError> {
        // Only auto-create for high-confidence correlations
        if correlation.confidence_score < 0.8 {
            return Ok(None);
        }
        
        let related_events = self.get_correlated_events(&correlation.correlation_id).await?;
        let incident_title = self.generate_incident_title(&related_events);
        let severity = self.calculate_incident_severity(&related_events);
        
        let incident = self.create_incident(CreateIncidentRequest {
            title: incident_title,
            description: Some(self.generate_incident_description(&related_events)),
            severity,
            created_by: Uuid::nil(), // System-generated
            event_ids: Some(related_events.iter().map(|e| e.id).collect()),
            tags: Some(vec!["auto-generated".to_string()]),
        }).await?;
        
        Ok(Some(incident))
    }
}
```

#### Incident Workflows
```rust
// src/core/security/incident_workflows.rs
pub struct IncidentWorkflow {
    db_pool: Arc<PgPool>,
}

impl IncidentWorkflow {
    pub async fn escalate_incident(&self, incident_id: Uuid, escalated_by: Uuid, reason: String) -> Result<(), SecurityError> {
        let mut incident = self.get_incident(incident_id).await?;
        
        // Record status change
        self.record_status_change(&incident, IncidentStatus::Escalated, escalated_by, Some(reason.clone())).await?;
        
        // Update incident
        incident.status = IncidentStatus::Escalated;
        incident.updated_at = Utc::now();
        self.update_incident(&incident).await?;
        
        // Trigger escalation notifications
        self.notification_service.incident_escalated(&incident, &reason).await?;
        
        Ok(())
    }
    
    pub async fn resolve_incident(&self, incident_id: Uuid, resolved_by: Uuid, resolution_notes: String) -> Result<(), SecurityError> {
        let mut incident = self.get_incident(incident_id).await?;
        
        // Validate resolution
        if incident.status == IncidentStatus::Closed {
            return Err(SecurityError::InvalidTransition("Cannot resolve closed incident"));
        }
        
        // Record resolution
        self.record_status_change(&incident, IncidentStatus::Resolved, resolved_by, Some(resolution_notes.clone())).await?;
        
        // Update incident
        incident.status = IncidentStatus::Resolved;
        incident.resolved_at = Some(Utc::now());
        incident.resolution_notes = Some(resolution_notes);
        incident.updated_at = Utc::now();
        
        self.update_incident(&incident).await?;
        
        // Notify stakeholders
        self.notification_service.incident_resolved(&incident).await?;
        
        Ok(())
    }
}
```

### 3. Real-time Notification Service

#### WebSocket Implementation
```rust
// src/core/security/websocket_service.rs
use actix_web_actors::ws;

pub struct SecurityWebSocket {
    user_id: Option<Uuid>,
    user_role: Option<UserRole>,
    subscriptions: HashSet<String>,
}

impl Actor for SecurityWebSocket {
    type Context = ws::WebsocketContext<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        // Join global security events room
        if let Some(user_id) = self.user_id {
            SecurityWebSocketManager::add_connection(user_id, ctx.address());
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SecurityWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                if let Ok(command) = serde_json::from_str::<WebSocketCommand>(&text) {
                    self.handle_command(command, ctx);
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl SecurityWebSocket {
    fn handle_command(&mut self, command: WebSocketCommand, ctx: &mut ws::WebsocketContext<Self>) {
        match command {
            WebSocketCommand::Subscribe { channel } => {
                if self.can_access_channel(&channel) {
                    self.subscriptions.insert(channel.clone());
                    let response = WebSocketResponse::Subscribed { channel };
                    ctx.text(serde_json::to_string(&response).unwrap());
                }
            }
            WebSocketCommand::Unsubscribe { channel } => {
                self.subscriptions.remove(&channel);
                let response = WebSocketResponse::Unsubscribed { channel };
                ctx.text(serde_json::to_string(&response).unwrap());
            }
        }
    }
}
```

---

## Real-time Processing Architecture

### Event Stream Processor

#### Core Processing Engine
```rust
// src/core/security/stream_processor.rs
pub struct SecurityEventStreamProcessor {
    event_receiver: UnboundedReceiver<SecurityEvent>,
    correlator: Arc<EventCorrelator>,
    incident_service: Arc<IncidentService>,
    notification_service: Arc<NotificationService>,
    rule_engine: Arc<RuleEngine>,
    metrics: Arc<ProcessingMetrics>,
}

impl SecurityEventStreamProcessor {
    pub async fn start_processing(&mut self) -> Result<(), SecurityError> {
        let mut batch_buffer = Vec::with_capacity(100);
        let mut last_flush = Instant::now();
        
        while let Some(event) = self.event_receiver.recv().await {
            batch_buffer.push(event);
            
            // Flush conditions
            let should_flush = batch_buffer.len() >= 100 
                || last_flush.elapsed() > Duration::from_secs(5)
                || batch_buffer.iter().any(|e| e.severity == EventSeverity::Critical);
                
            if should_flush {
                self.process_batch(mem::take(&mut batch_buffer)).await?;
                last_flush = Instant::now();
            }
        }
        
        Ok(())
    }
    
    async fn process_batch(&self, events: Vec<SecurityEvent>) -> Result<(), SecurityError> {
        let start = Instant::now();
        
        // Parallel processing for different event types
        let futures = events.chunks(10).map(|chunk| {
            self.process_event_chunk(chunk.to_vec())
        }).collect::<Vec<_>>();
        
        try_join_all(futures).await?;
        
        self.metrics.record_batch_processing_time(start.elapsed());
        Ok(())
    }
    
    async fn process_event_chunk(&self, events: Vec<SecurityEvent>) -> Result<(), SecurityError> {
        for event in events {
            // Apply security rules
            let rule_results = self.rule_engine.evaluate_event(&event).await?;
            
            // Check for correlations
            if let Some(correlation) = self.correlator.find_correlations(&event).await? {
                // Auto-create incident if correlation confidence is high
                if let Some(incident) = self.incident_service.auto_create_from_correlation(correlation).await? {
                    self.notification_service.new_incident_auto_created(&incident).await?;
                }
            }
            
            // Process rule-based actions
            for rule_result in rule_results {
                self.execute_rule_action(&event, rule_result).await?;
            }
            
            // Send real-time notifications
            self.notification_service.event_processed(&event).await?;
        }
        
        Ok(())
    }
}
```

#### Rule Engine
```rust
// src/core/security/rule_engine.rs
pub struct RuleEngine {
    rules: Arc<RwLock<Vec<SecurityRule>>>,
    db_pool: Arc<PgPool>,
}

#[derive(Debug, Clone)]
pub struct SecurityRule {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub enabled: bool,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub enum RuleCondition {
    EventType(Vec<SecurityEventType>),
    Severity(Vec<EventSeverity>),
    SourceIP(Vec<IpAddr>),
    UserBased(Vec<Uuid>),
    FrequencyThreshold { 
        count: u32, 
        time_window: Duration 
    },
    Complex(Box<ComplexCondition>),
}

impl RuleEngine {
    pub async fn evaluate_event(&self, event: &SecurityEvent) -> Result<Vec<RuleResult>, SecurityError> {
        let rules = self.rules.read().await;
        let mut results = Vec::new();
        
        for rule in rules.iter().filter(|r| r.enabled) {
            if self.evaluate_condition(&rule.condition, event).await? {
                results.push(RuleResult {
                    rule_id: rule.id,
                    action: rule.action.clone(),
                    priority: rule.priority,
                    triggered_at: Utc::now(),
                });
            }
        }
        
        // Sort by priority (higher number = higher priority)
        results.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(results)
    }
    
    async fn evaluate_condition(&self, condition: &RuleCondition, event: &SecurityEvent) -> Result<bool, SecurityError> {
        match condition {
            RuleCondition::EventType(types) => Ok(types.contains(&event.event_type)),
            RuleCondition::Severity(severities) => Ok(severities.contains(&event.severity)),
            RuleCondition::SourceIP(ips) => {
                if let Some(source_ip) = event.source_ip {
                    Ok(ips.contains(&source_ip))
                } else {
                    Ok(false)
                }
            },
            RuleCondition::FrequencyThreshold { count, time_window } => {
                self.check_frequency_threshold(event, *count, *time_window).await
            },
            RuleCondition::Complex(complex) => {
                self.evaluate_complex_condition(complex, event).await
            },
            _ => Ok(false),
        }
    }
}
```

### WebSocket Security and Management

#### Connection Security
```rust
// src/core/security/websocket_security.rs
pub struct WebSocketSecurity {
    active_connections: Arc<Mutex<HashMap<Uuid, ConnectionInfo>>>,
    rate_limiter: Arc<RateLimiter>,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub user_id: Uuid,
    pub user_role: UserRole,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub message_count: u64,
    pub subscriptions: HashSet<String>,
}

impl WebSocketSecurity {
    pub async fn authenticate_connection(&self, token: &str) -> Result<ConnectionInfo, SecurityError> {
        // Validate JWT token
        let claims = self.jwt_service.validate_token(token).await?;
        
        // Check if user is active
        let user = self.user_service.get_user(claims.user_id).await?;
        if !user.is_active {
            return Err(SecurityError::UserInactive);
        }
        
        // Rate limiting check
        if !self.rate_limiter.check_connection_rate(claims.user_id).await? {
            return Err(SecurityError::RateLimitExceeded);
        }
        
        Ok(ConnectionInfo {
            user_id: claims.user_id,
            user_role: user.role,
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            message_count: 0,
            subscriptions: HashSet::new(),
        })
    }
    
    pub async fn check_message_rate_limit(&self, user_id: Uuid) -> Result<bool, SecurityError> {
        let mut connections = self.active_connections.lock().await;
        
        if let Some(conn_info) = connections.get_mut(&user_id) {
            // Check message rate (max 60 messages per minute)
            let now = Utc::now();
            let one_minute_ago = now - chrono::Duration::minutes(1);
            
            if conn_info.last_activity < one_minute_ago {
                conn_info.message_count = 0;
            }
            
            if conn_info.message_count >= 60 {
                return Ok(false);
            }
            
            conn_info.message_count += 1;
            conn_info.last_activity = now;
            Ok(true)
        } else {
            Ok(false) // Connection not found
        }
    }
    
    pub async fn can_subscribe_to_channel(&self, user_id: Uuid, channel: &str) -> Result<bool, SecurityError> {
        let connections = self.active_connections.lock().await;
        
        if let Some(conn_info) = connections.get(&user_id) {
            match channel {
                "security.events.critical" => Ok(conn_info.user_role == UserRole::Admin),
                "security.incidents.all" => Ok(matches!(conn_info.user_role, UserRole::Admin | UserRole::SecurityAnalyst)),
                "security.events.own" => Ok(true), // Users can see their own events
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}
```

---

## Event Correlation Engine

### Correlation Algorithms

#### Temporal Correlation
```rust
// src/core/security/correlation/temporal.rs
pub struct TemporalCorrelator {
    db_pool: Arc<PgPool>,
    time_windows: HashMap<SecurityEventType, Duration>,
}

impl TemporalCorrelator {
    pub async fn find_temporal_correlations(&self, event: &SecurityEvent) -> Result<Vec<EventCorrelation>, SecurityError> {
        let time_window = self.get_time_window(event.event_type);
        let start_time = event.timestamp - time_window;
        let end_time = event.timestamp + time_window;
        
        // Find events within time window
        let related_events = sqlx::query_as!(
            SecurityEvent,
            r#"
            SELECT * FROM security_events 
            WHERE timestamp BETWEEN $1 AND $2
            AND id != $3
            AND (
                -- Same user, different actions
                (user_id = $4 AND event_type != $5) OR
                -- Same source IP, different users
                (source_ip = $6 AND user_id != $4) OR
                -- Related event types
                event_type = ANY($7)
            )
            ORDER BY timestamp
            "#,
            start_time,
            end_time,
            event.id,
            event.user_id,
            event.event_type as SecurityEventType,
            event.source_ip,
            &self.get_related_event_types(event.event_type)
        )
        .fetch_all(&*self.db_pool)
        .await?;
        
        let mut correlations = Vec::new();
        
        for related_event in related_events {
            let confidence = self.calculate_temporal_confidence(event, &related_event);
            
            if confidence >= 0.5 {
                correlations.push(EventCorrelation {
                    id: Uuid::new_v4(),
                    primary_event_id: event.id,
                    related_event_id: related_event.id,
                    correlation_type: CorrelationType::Temporal,
                    confidence_score: confidence,
                    created_at: Utc::now(),
                });
            }
        }
        
        Ok(correlations)
    }
    
    fn calculate_temporal_confidence(&self, primary: &SecurityEvent, related: &SecurityEvent) -> f64 {
        let time_diff = (primary.timestamp - related.timestamp).num_seconds().abs() as f64;
        let max_time = self.get_time_window(primary.event_type).as_secs() as f64;
        
        // Base confidence from time proximity
        let time_confidence = 1.0 - (time_diff / max_time).min(1.0);
        
        // Boost confidence for related patterns
        let pattern_boost = match (primary.event_type, related.event_type) {
            (SecurityEventType::AuthenticationFailure, SecurityEventType::AccountLockout) => 0.8,
            (SecurityEventType::SuspiciousLogin, SecurityEventType::DataAccess) => 0.7,
            (SecurityEventType::PrivilegeEscalation, SecurityEventType::DataModification) => 0.9,
            _ => 0.0,
        };
        
        (time_confidence * 0.6 + pattern_boost * 0.4).min(1.0)
    }
}
```

#### Pattern-based Correlation
```rust
// src/core/security/correlation/pattern.rs
pub struct PatternCorrelator {
    patterns: Arc<RwLock<Vec<AttackPattern>>>,
    db_pool: Arc<PgPool>,
}

#[derive(Debug, Clone)]
pub struct AttackPattern {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub event_sequence: Vec<SecurityEventType>,
    pub max_time_span: Duration,
    pub min_confidence: f64,
}

impl PatternCorrelator {
    pub async fn find_pattern_correlations(&self, event: &SecurityEvent) -> Result<Vec<EventCorrelation>, SecurityError> {
        let patterns = self.patterns.read().await;
        let mut correlations = Vec::new();
        
        for pattern in patterns.iter() {
            if let Some(correlation) = self.match_pattern(event, pattern).await? {
                correlations.push(correlation);
            }
        }
        
        Ok(correlations)
    }
    
    async fn match_pattern(&self, event: &SecurityEvent, pattern: &AttackPattern) -> Result<Option<EventCorrelation>, SecurityError> {
        // Check if this event type is part of the pattern
        if !pattern.event_sequence.contains(&event.event_type) {
            return Ok(None);
        }
        
        // Look for preceding events in the pattern
        let lookback_time = event.timestamp - pattern.max_time_span;
        
        let preceding_events = sqlx::query_as!(
            SecurityEvent,
            r#"
            SELECT * FROM security_events 
            WHERE timestamp BETWEEN $1 AND $2
            AND (user_id = $3 OR source_ip = $4)
            AND event_type = ANY($5)
            ORDER BY timestamp
            "#,
            lookback_time,
            event.timestamp,
            event.user_id,
            event.source_ip,
            &pattern.event_sequence.iter().map(|t| *t as SecurityEventType).collect::<Vec<_>>()
        )
        .fetch_all(&*self.db_pool)
        .await?;
        
        // Check if we have a sequence match
        let confidence = self.calculate_sequence_confidence(&preceding_events, event, pattern);
        
        if confidence >= pattern.min_confidence {
            Ok(Some(EventCorrelation {
                id: Uuid::new_v4(),
                primary_event_id: event.id,
                related_event_id: preceding_events.last().unwrap().id,
                correlation_type: CorrelationType::PatternBased,
                confidence_score: confidence,
                created_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }
}
```

### Machine Learning Enhancement
```rust
// src/core/security/correlation/ml_correlator.rs
pub struct MLCorrelator {
    model: Arc<Mutex<Option<AnomalyModel>>>,
    feature_extractor: FeatureExtractor,
}

impl MLCorrelator {
    pub async fn train_model(&self, historical_events: Vec<SecurityEvent>) -> Result<(), SecurityError> {
        let features = self.feature_extractor.extract_features(&historical_events);
        
        // Use isolation forest for anomaly detection
        let model = IsolationForest::new()
            .with_contamination(0.1)
            .with_n_estimators(100)
            .fit(features)?;
            
        *self.model.lock().await = Some(model);
        Ok(())
    }
    
    pub async fn predict_anomaly(&self, event: &SecurityEvent) -> Result<f64, SecurityError> {
        let model_guard = self.model.lock().await;
        
        if let Some(model) = model_guard.as_ref() {
            let features = self.feature_extractor.extract_single_feature(event);
            let anomaly_score = model.predict(&features)?;
            Ok(anomaly_score)
        } else {
            Err(SecurityError::ModelNotTrained)
        }
    }
}

pub struct FeatureExtractor;

impl FeatureExtractor {
    pub fn extract_single_feature(&self, event: &SecurityEvent) -> Vec<f64> {
        vec![
            event.event_type as u8 as f64,
            event.severity as u8 as f64,
            event.timestamp.timestamp() as f64,
            event.user_id.map_or(0.0, |_| 1.0),
            event.source_ip.map_or(0.0, |ip| match ip {
                IpAddr::V4(ipv4) => ipv4.octets()[3] as f64,
                IpAddr::V6(_) => 255.0,
            }),
            // Add more features as needed
        ]
    }
}
```

---

## Security Considerations

### Access Control Matrix

| Resource | Admin | Security Analyst | Regular User |
|----------|--------|------------------|--------------|
| View All Incidents | ✓ | ✓ | ✗ |
| Create Incidents | ✓ | ✓ | ✗ |
| Modify Incidents | ✓ | ✓ (assigned only) | ✗ |
| Delete Incidents | ✓ | ✗ | ✗ |
| View All Events | ✓ | ✓ | ✗ |
| View Own Events | ✓ | ✓ | ✓ |
| System Configuration | ✓ | ✗ | ✗ |
| User Management | ✓ | ✗ | ✗ |
| Real-time Alerts | ✓ | ✓ | Limited |

### Security Implementation

#### Input Validation and Sanitization
```rust
// src/common/validation/security_validation.rs
pub struct SecurityValidator;

impl SecurityValidator {
    pub fn validate_event_input(event: &CreateSecurityEventRequest) -> Result<(), ValidationError> {
        // Validate event type
        if !Self::is_valid_event_type(&event.event_type) {
            return Err(ValidationError::InvalidEventType);
        }
        
        // Sanitize text fields
        let sanitized_details = Self::sanitize_json(&event.details)?;
        
        // Validate IP address format
        if let Some(ip) = &event.source_ip {
            if !Self::is_valid_ip_address(ip) {
                return Err(ValidationError::InvalidIpAddress);
            }
        }
        
        // Check for injection attempts
        if Self::contains_sql_injection_patterns(&event.details) {
            return Err(ValidationError::PotentialInjection);
        }
        
        Ok(())
    }
    
    fn sanitize_json(value: &serde_json::Value) -> Result<serde_json::Value, ValidationError> {
        match value {
            serde_json::Value::String(s) => {
                let sanitized = s
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\'', "&#x27;");
                Ok(serde_json::Value::String(sanitized))
            },
            serde_json::Value::Object(map) => {
                let mut sanitized_map = serde_json::Map::new();
                for (key, val) in map {
                    sanitized_map.insert(key.clone(), Self::sanitize_json(val)?);
                }
                Ok(serde_json::Value::Object(sanitized_map))
            },
            _ => Ok(value.clone()),
        }
    }
}
```

#### Rate Limiting and DDoS Protection
```rust
// src/infrastructure/middleware/rate_limiting.rs
pub struct RateLimitingMiddleware {
    redis_pool: Arc<RedisPool>,
    limits: HashMap<String, RateLimit>,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_window: u32,
    pub window_seconds: u32,
}

impl RateLimitingMiddleware {
    pub async fn check_rate_limit(&self, key: &str, limit: &RateLimit) -> Result<bool, RedisError> {
        let mut conn = self.redis_pool.get().await?;
        
        let current_count: Option<u32> = conn.get(key).await?;
        
        match current_count {
            Some(count) if count >= limit.requests_per_window => Ok(false),
            Some(count) => {
                conn.incr(key, 1).await?;
                Ok(true)
            },
            None => {
                conn.set_ex(key, 1, limit.window_seconds as u64).await?;
                Ok(true)
            },
        }
    }
}
```

#### Encryption and Data Protection
```rust
// src/core/security/encryption.rs
pub struct DataProtection {
    encryption_key: Arc<[u8; 32]>,
    nonce_generator: Arc<Mutex<SystemRandom>>,
}

impl DataProtection {
    pub fn encrypt_sensitive_data(&self, data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        let mut nonce = [0u8; 12];
        self.nonce_generator.lock().unwrap().fill(&mut nonce)?;
        
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)?;
        let sealing_key = SealingKey::new(unbound_key, Nonce::from(nonce));
        
        let mut encrypted_data = data.to_vec();
        sealing_key.seal_in_place(Aad::empty(), &mut encrypted_data)?;
        
        // Prepend nonce to encrypted data
        let mut result = nonce.to_vec();
        result.extend_from_slice(&encrypted_data);
        
        Ok(result)
    }
    
    pub fn decrypt_sensitive_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if encrypted_data.len() < 12 {
            return Err(EncryptionError::InvalidData);
        }
        
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)?;
        let opening_key = OpeningKey::new(unbound_key, nonce);
        
        let mut decrypted_data = ciphertext.to_vec();
        let plaintext = opening_key.open_in_place(Aad::empty(), &mut decrypted_data)?;
        
        Ok(plaintext.to_vec())
    }
}
```

---

## Performance Optimization

### Database Query Optimization

#### Query Performance Monitoring
```rust
// src/infrastructure/database/query_monitor.rs
pub struct QueryMonitor {
    slow_query_threshold: Duration,
    metrics_collector: Arc<MetricsCollector>,
}

impl QueryMonitor {
    pub async fn execute_monitored_query<T, F>(&self, query_name: &str, f: F) -> Result<T, sqlx::Error>
    where
        F: Future<Output = Result<T, sqlx::Error>>,
    {
        let start = Instant::now();
        let result = f.await;
        let duration = start.elapsed();
        
        // Log slow queries
        if duration > self.slow_query_threshold {
            warn!("Slow query detected: {} took {:?}", query_name, duration);
        }
        
        // Collect metrics
        self.metrics_collector.record_query_duration(query_name, duration);
        
        result
    }
}
```

#### Optimized Repository Implementations
```rust
// src/core/security/repositories/security_event_repository.rs
pub struct SecurityEventRepository {
    db_pool: Arc<PgPool>,
    query_monitor: Arc<QueryMonitor>,
}

impl SecurityEventRepository {
    pub async fn get_events_with_pagination(&self, filters: EventFilters, page: u32, page_size: u32) -> Result<(Vec<SecurityEvent>, u64), DatabaseError> {
        let offset = (page - 1) * page_size;
        
        // Use prepared statement for better performance
        let query = self.build_filtered_query(&filters);
        
        self.query_monitor.execute_monitored_query("get_events_paginated", async {
            // Count total for pagination
            let count_result = sqlx::query_scalar::<_, i64>(&query.count_query)
                .bind_all(&query.params)
                .fetch_one(&*self.db_pool)
                .await?;
                
            // Fetch actual data
            let events = sqlx::query_as::<_, SecurityEvent>(&query.select_query)
                .bind_all(&query.params)
                .bind(page_size as i64)
                .bind(offset as i64)
                .fetch_all(&*self.db_pool)
                .await?;
                
            Ok((events, count_result as u64))
        }).await
    }
    
    pub async fn bulk_insert_events(&self, events: Vec<SecurityEvent>) -> Result<(), DatabaseError> {
        if events.is_empty() {
            return Ok(());
        }
        
        self.query_monitor.execute_monitored_query("bulk_insert_events", async {
            let mut tx = self.db_pool.begin().await?;
            
            // Use COPY for bulk inserts (fastest method)
            let mut copy_stmt = tx.copy_in_raw(
                "COPY security_events (id, event_type, severity, source_ip, user_id, timestamp, details, raw_log, created_at) FROM STDIN WITH (FORMAT BINARY)"
            ).await?;
            
            for event in events {
                // Serialize event data to binary format
                let row_data = self.serialize_event_for_copy(&event)?;
                copy_stmt.send(row_data).await?;
            }
            
            copy_stmt.finish().await?;
            tx.commit().await?;
            
            Ok(())
        }).await
    }
}
```

### Caching Strategy

#### Multi-layer Caching
```rust
// src/infrastructure/cache/cache_manager.rs
pub struct CacheManager {
    redis_pool: Arc<RedisPool>,
    local_cache: Arc<Mutex<LruCache<String, CachedValue>>>,
    cache_config: CacheConfig,
}

#[derive(Debug, Clone)]
pub struct CachedValue {
    data: Vec<u8>,
    expires_at: DateTime<Utc>,
    version: u64,
}

impl CacheManager {
    pub async fn get_or_compute<T, F>(&self, key: &str, ttl: Duration, compute_fn: F) -> Result<T, CacheError>
    where
        T: Serialize + DeserializeOwned,
        F: Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    {
        // Try local cache first (L1)
        if let Some(cached) = self.get_from_local_cache::<T>(key)? {
            return Ok(cached);
        }
        
        // Try Redis cache (L2)
        if let Some(cached) = self.get_from_redis_cache::<T>(key).await? {
            // Store in local cache for next time
            self.store_in_local_cache(key, &cached, ttl)?;
            return Ok(cached);
        }
        
        // Compute value
        let computed_value = compute_fn.await?;
        
        // Store in both cache layers
        self.store_in_redis_cache(key, &computed_value, ttl).await?;
        self.store_in_local_cache(key, &computed_value, ttl)?;
        
        Ok(computed_value)
    }
    
    pub async fn invalidate(&self, pattern: &str) -> Result<(), CacheError> {
        // Invalidate Redis cache
        let mut conn = self.redis_pool.get().await?;
        let keys: Vec<String> = conn.keys(pattern).await?;
        
        if !keys.is_empty() {
            conn.del(&keys).await?;
        }
        
        // Invalidate local cache
        let mut local = self.local_cache.lock().await;
        local.clear(); // Simple approach - clear all local cache
        
        Ok(())
    }
}
```

### Connection Pool Optimization
```rust
// src/infrastructure/database/pool_config.rs
pub fn create_optimized_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(50) // Adjust based on system capacity
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(3600))
        .before_acquire(|conn, meta| {
            Box::pin(async move {
                // Log connection pool metrics
                if meta.age > Duration::from_secs(60) {
                    debug!("Using pooled connection aged {:?}", meta.age);
                }
                Ok(())
            })
        })
        .connect(database_url)
}
```

---

## Implementation Timeline

### Revised Realistic Timeline: 16-20 weeks

#### Phase 1: Foundation and Schema (Weeks 1-5)
**Week 1-2: Database Schema Implementation**
- [ ] Create database migration files with proper table ordering
- [ ] Implement custom types and enums
- [ ] Add comprehensive indexing strategy
- [ ] Set up database partitioning for events table
- [ ] Create automated cleanup procedures

**Week 3-4: Core Models and Repositories**
- [ ] Implement SecurityEvent and SecurityIncident models
- [ ] Create repository layer with optimized queries
- [ ] Add bulk insert operations for events
- [ ] Implement pagination and filtering
- [ ] Add repository unit tests

**Week 5: Basic Services Foundation**
- [ ] Create EventCollector service structure
- [ ] Implement IncidentService basic operations
- [ ] Add validation layer for all inputs
- [ ] Create error handling framework
- [ ] Set up logging infrastructure

#### Phase 2: Event Collection and Processing (Weeks 6-10)
**Week 6-7: Event Collection System**
- [ ] Implement event collection middleware
- [ ] Create buffered event processing
- [ ] Add event validation and sanitization
- [ ] Implement rate limiting for event ingestion
- [ ] Add metrics collection for processing

**Week 8-9: Real-time Processing Engine**
- [ ] Create async event stream processor
- [ ] Implement batch processing logic
- [ ] Add event queue management
- [ ] Create processing metrics and monitoring
- [ ] Implement error recovery mechanisms

**Week 10: Rule Engine Foundation**
- [ ] Create rule definition structure
- [ ] Implement basic rule evaluation
- [ ] Add rule-based actions
- [ ] Create rule management interface
- [ ] Add rule testing framework

#### Phase 3: Correlation and Intelligence (Weeks 11-14)
**Week 11-12: Event Correlation Engine**
- [ ] Implement temporal correlation algorithms
- [ ] Create pattern-based correlation
- [ ] Add user-based correlation logic
- [ ] Implement correlation confidence scoring
- [ ] Create correlation persistence layer

**Week 13: Advanced Correlation**
- [ ] Add IP-based correlation
- [ ] Implement frequency-based detection
- [ ] Create complex condition evaluation
- [ ] Add machine learning correlation preparation
- [ ] Implement correlation performance optimization

**Week 14: Automated Incident Creation**
- [ ] Create auto-incident creation logic
- [ ] Implement correlation-to-incident mapping
- [ ] Add incident severity calculation
- [ ] Create automated incident descriptions
- [ ] Add manual override capabilities

#### Phase 4: Real-time Features and UI Integration (Weeks 15-18)
**Week 15-16: WebSocket Implementation**
- [ ] Create secure WebSocket connections
- [ ] Implement connection authentication
- [ ] Add rate limiting for WebSocket messages
- [ ] Create subscription management
- [ ] Implement real-time event broadcasting

**Week 17: Dashboard Integration**
- [ ] Update frontend to consume real API endpoints
- [ ] Implement real-time dashboard updates
- [ ] Add WebSocket connection management
- [ ] Create error handling for connection issues
- [ ] Add offline/reconnection handling

**Week 18: Advanced UI Features**
- [ ] Implement incident creation from UI
- [ ] Add event correlation visualization
- [ ] Create incident workflow management
- [ ] Add bulk operations support
- [ ] Implement advanced filtering and search

#### Phase 5: Security and Performance (Weeks 19-20)
**Week 19: Security Hardening**
- [ ] Implement comprehensive input validation
- [ ] Add SQL injection protection
- [ ] Create audit logging for all operations
- [ ] Implement data encryption for sensitive fields
- [ ] Add comprehensive access control tests

**Week 20: Performance Optimization and Testing**
- [ ] Implement caching strategy
- [ ] Optimize database queries
- [ ] Add performance monitoring
- [ ] Create load testing scenarios
- [ ] Final integration testing and bug fixes

### Detailed Task Breakdown

#### Critical Dependencies
1. **Database Schema** must be completed before any service implementation
2. **Basic Services** must exist before correlation engine
3. **Event Collection** must be stable before real-time features
4. **WebSocket Security** must be implemented before frontend integration

#### Resource Requirements
- **Database Administrator**: 20% time for schema optimization
- **Backend Developer**: Full-time for service implementation
- **Frontend Developer**: 50% time for UI integration
- **Security Specialist**: 30% time for security review
- **DevOps Engineer**: 40% time for deployment and monitoring

#### Risk Mitigation Strategies
1. **Weekly milestone reviews** to catch issues early
2. **Parallel development** where dependencies allow
3. **Automated testing** at each phase
4. **Performance benchmarking** at each major milestone
5. **Security reviews** at phase completion

---

## Testing Strategy

### Comprehensive Testing Framework

#### Unit Testing Strategy
```rust
// tests/unit/security/test_event_collector.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_event_collection_buffering() {
        let collector = create_test_event_collector().await;
        let events = create_test_events(50);
        
        for event in events {
            collector.collect_event(event).await.unwrap();
        }
        
        // Verify buffer management
        assert_eq!(collector.get_buffer_size().await, 0); // Should be flushed
        
        // Verify events were persisted
        let persisted_count = collector.get_event_count().await.unwrap();
        assert_eq!(persisted_count, 50);
    }
    
    #[tokio::test]
    async fn test_critical_event_immediate_processing() {
        let collector = create_test_event_collector().await;
        let critical_event = SecurityEvent {
            severity: EventSeverity::Critical,
            ..create_test_event()
        };
        
        let start = Instant::now();
        collector.collect_event(critical_event).await.unwrap();
        let duration = start.elapsed();
        
        // Critical events should be processed immediately (< 100ms)
        assert!(duration < Duration::from_millis(100));
    }
}
```

#### Integration Testing
```rust
// tests/integration/security/test_incident_workflow.rs
#[tokio::test]
async fn test_complete_incident_lifecycle() {
    let test_context = create_test_context().await;
    
    // Step 1: Create security events
    let events = vec![
        create_auth_failure_event(),
        create_suspicious_login_event(),
        create_account_lockout_event(),
    ];
    
    for event in &events {
        test_context.event_collector.collect_event(event.clone()).await.unwrap();
    }
    
    // Wait for processing
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Step 2: Verify correlation was created
    let correlations = test_context.correlation_service
        .get_correlations_for_events(&events.iter().map(|e| e.id).collect()).await.unwrap();
    assert!(!correlations.is_empty());
    
    // Step 3: Verify auto-incident creation
    let incidents = test_context.incident_service
        .get_incidents_by_status(IncidentStatus::Open).await.unwrap();
    assert_eq!(incidents.len(), 1);
    
    let incident = &incidents[0];
    assert!(incident.title.contains("Authentication"));
    
    // Step 4: Test incident resolution
    test_context.incident_service
        .resolve_incident(
            incident.id,
            test_context.admin_user.id,
            "False positive - user forgot password".to_string()
        ).await.unwrap();
    
    // Step 5: Verify resolution
    let resolved_incident = test_context.incident_service
        .get_incident(incident.id).await.unwrap();
    assert_eq!(resolved_incident.status, IncidentStatus::Resolved);
    assert!(resolved_incident.resolved_at.is_some());
}
```

#### Performance Testing
```rust
// tests/performance/security/test_event_processing.rs
#[tokio::test]
async fn test_high_volume_event_processing() {
    let test_context = create_performance_test_context().await;
    let event_count = 10_000;
    
    // Generate test events
    let events: Vec<_> = (0..event_count)
        .map(|i| create_test_event_with_id(i))
        .collect();
    
    let start = Instant::now();
    
    // Process events in parallel batches
    let batch_size = 100;
    let batches: Vec<_> = events.chunks(batch_size).collect();
    
    let futures = batches.into_iter().map(|batch| {
        let collector = test_context.event_collector.clone();
        async move {
            for event in batch {
                collector.collect_event(event.clone()).await.unwrap();
            }
        }
    });
    
    futures::future::join_all(futures).await;
    
    let processing_time = start.elapsed();
    let events_per_second = event_count as f64 / processing_time.as_secs_f64();
    
    // Assert performance requirements
    assert!(events_per_second > 1000.0, "Processing rate too slow: {} events/sec", events_per_second);
    assert!(processing_time < Duration::from_secs(30), "Total processing time too long: {:?}", processing_time);
    
    // Verify all events were processed
    tokio::time::sleep(Duration::from_secs(5)).await; // Allow processing to complete
    let processed_count = test_context.get_processed_event_count().await.unwrap();
    assert_eq!(processed_count, event_count);
}
```

#### Load Testing with Realistic Scenarios
```rust
// tests/load/security/test_concurrent_operations.rs
#[tokio::test]
async fn test_concurrent_dashboard_access() {
    let test_context = create_load_test_context().await;
    
    // Simulate 50 concurrent users accessing dashboard
    let concurrent_users = 50;
    let operations_per_user = 20;
    
    let futures = (0..concurrent_users).map(|user_id| {
        let context = test_context.clone();
        async move {
            let user_token = context.create_test_user_token(user_id).await;
            
            for _ in 0..operations_per_user {
                // Mix of operations
                let operation = rand::random::<u8>() % 4;
                match operation {
                    0 => context.get_incidents_paginated(&user_token, 1, 20).await,
                    1 => context.get_security_events(&user_token, Default::default()).await,
                    2 => context.get_incident_details(&user_token, random_incident_id()).await,
                    3 => context.search_events(&user_token, "authentication").await,
                    _ => unreachable!(),
                }.unwrap();
                
                // Small delay between operations
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    });
    
    let start = Instant::now();
    futures::future::join_all(futures).await;
    let total_time = start.elapsed();
    
    // Performance assertions
    assert!(total_time < Duration::from_secs(60), "Load test took too long: {:?}", total_time);
    
    // Verify system stability
    let health_check = test_context.perform_health_check().await.unwrap();
    assert!(health_check.database_healthy);
    assert!(health_check.cache_healthy);
    assert!(health_check.processing_healthy);
}
```

### Testing Metrics and Coverage

#### Code Coverage Requirements
- **Unit Tests**: Minimum 90% code coverage
- **Integration Tests**: Cover all critical user workflows  
- **Performance Tests**: Validate all performance requirements
- **Security Tests**: Test all attack vectors and access controls

#### Test Data Management
```rust
// tests/common/test_data_factory.rs
pub struct TestDataFactory {
    db_pool: Arc<PgPool>,
}

impl TestDataFactory {
    pub async fn create_realistic_event_dataset(&self, count: usize) -> Vec<SecurityEvent> {
        let event_types = [
            SecurityEventType::AuthenticationFailure,
            SecurityEventType::AuthenticationSuccess,
            SecurityEventType::DataAccess,
            SecurityEventType::SuspiciousLogin,
        ];
        
        let mut events = Vec::with_capacity(count);
        let base_time = Utc::now() - chrono::Duration::hours(24);
        
        for i in 0..count {
            let event_type = event_types[i % event_types.len()];
            let timestamp = base_time + chrono::Duration::minutes(i as i64 * 5);
            
            events.push(SecurityEvent {
                id: Uuid::new_v4(),
                event_type,
                severity: self.calculate_realistic_severity(event_type),
                source_ip: self.generate_realistic_ip(),
                user_id: self.get_random_test_user_id(),
                timestamp,
                details: self.generate_realistic_details(event_type),
                ..Default::default()
            });
        }
        
        events
    }
}
```

---

## Success Criteria

### Functional Requirements

#### Core Functionality
1. **Event Collection**
   - ✅ System can collect 10,000+ events per minute
   - ✅ Events are validated and sanitized before storage
   - ✅ Critical events are processed within 100ms
   - ✅ Buffer management prevents data loss

2. **Event Correlation**
   - ✅ Temporal correlations are identified within time windows
   - ✅ Pattern-based correlations achieve >80% accuracy
   - ✅ User and IP-based correlations are detected
   - ✅ Confidence scoring accurately reflects correlation strength

3. **Incident Management**
   - ✅ Incidents can be created, updated, and resolved
   - ✅ Auto-incident creation from high-confidence correlations
   - ✅ Complete audit trail for all incident changes
   - ✅ Workflow management with proper state transitions

4. **Real-time Features**
   - ✅ Dashboard updates in real-time via WebSocket
   - ✅ Connection authentication and rate limiting
   - ✅ Subscription management for different event types
   - ✅ Graceful handling of connection failures

### Performance Requirements

#### Response Time Targets
- **Dashboard Load**: < 2 seconds for 1000 events
- **Event Search**: < 1 second for filtered results
- **Incident Creation**: < 500ms for manual creation
- **Real-time Updates**: < 100ms latency for WebSocket updates

#### Throughput Requirements
- **Event Processing**: 10,000 events/minute sustained
- **Concurrent Users**: 100 simultaneous dashboard users
- **Database Queries**: < 100ms for 95th percentile
- **Memory Usage**: < 2GB for typical workload

#### Scalability Targets
```rust
// Performance benchmarks to validate
struct PerformanceBenchmarks {
    events_per_second: u32,        // Target: 167 (10k/min)
    concurrent_users: u32,         // Target: 100
    database_response_time: u32,   // Target: <100ms (95th percentile)
    memory_usage_mb: u32,          // Target: <2048MB
    cpu_usage_percent: f32,        // Target: <80%
}

// Automated performance validation
#[tokio::test]
async fn validate_performance_benchmarks() {
    let metrics = run_performance_test_suite().await.unwrap();
    
    assert!(metrics.events_per_second >= 167, "Event processing rate below target");
    assert!(metrics.concurrent_users >= 100, "Concurrent user capacity below target");
    assert!(metrics.database_response_time <= 100, "Database response time above target");
    assert!(metrics.memory_usage_mb <= 2048, "Memory usage above target");
    assert!(metrics.cpu_usage_percent <= 80.0, "CPU usage above target");
}
```

### Security Requirements

#### Access Control Validation
- ✅ Role-based access control properly restricts functionality
- ✅ JWT tokens are properly validated and revoked when necessary
- ✅ All sensitive operations are logged in audit trail
- ✅ Input validation prevents injection attacks

#### Data Protection
- ✅ Sensitive data is encrypted at rest and in transit
- ✅ Personal information is properly anonymized in logs
- ✅ Data retention policies are automatically enforced
- ✅ Backup and recovery procedures are tested

### Operational Requirements

#### Monitoring and Alerting
```rust
// Key metrics to monitor
struct OperationalMetrics {
    // Processing metrics
    event_processing_rate: f64,
    event_processing_errors: u64,
    correlation_accuracy: f64,
    incident_auto_creation_rate: f64,
    
    // System health
    database_connection_pool_usage: f32,
    cache_hit_ratio: f32,
    websocket_connection_count: u32,
    api_error_rate: f32,
    
    // Business metrics
    incidents_created_per_hour: u32,
    incidents_resolved_per_hour: u32,
    average_incident_resolution_time: Duration,
    false_positive_rate: f32,
}
```

#### Deployment and Maintenance
- ✅ Zero-downtime deployment capability
- ✅ Automated database migrations
- ✅ Health check endpoints for monitoring
- ✅ Proper logging and error tracking
- ✅ Performance metrics collection

---

## Risk Assessment

### Technical Risks

#### High Priority Risks

1. **Database Performance Degradation**
   - **Risk**: Large event volumes cause query performance issues
   - **Mitigation**: Comprehensive indexing, partitioning, query optimization
   - **Monitoring**: Query execution time alerts, slow query logging
   - **Contingency**: Read replicas, query result caching

2. **Event Processing Bottlenecks**
   - **Risk**: Event ingestion rate exceeds processing capacity
   - **Mitigation**: Async processing, buffer management, backpressure handling
   - **Monitoring**: Queue depth alerts, processing lag metrics
   - **Contingency**: Horizontal scaling, priority-based processing

3. **WebSocket Connection Scaling**
   - **Risk**: High number of concurrent connections exhausts resources
   - **Mitigation**: Connection limits, rate limiting, connection pooling
   - **Monitoring**: Connection count metrics, memory usage alerts
   - **Contingency**: Connection clustering, graceful degradation

#### Medium Priority Risks

4. **False Positive Correlations**
   - **Risk**: Correlation engine generates too many false positives
   - **Mitigation**: Confidence scoring, manual tuning capabilities, ML improvements
   - **Monitoring**: False positive rate tracking, user feedback collection
   - **Contingency**: Manual correlation override, rule refinement

5. **Memory Usage Growth**
   - **Risk**: System memory usage grows unbounded over time
   - **Mitigation**: Proper resource cleanup, connection limits, cache eviction
   - **Monitoring**: Memory usage alerts, garbage collection metrics
   - **Contingency**: Automatic restart procedures, memory debugging tools

### Security Risks

#### Critical Security Concerns

1. **Privilege Escalation**
   - **Risk**: Users gain unauthorized access to admin functions
   - **Mitigation**: Comprehensive role validation, audit logging, least privilege principle
   - **Detection**: Unusual access pattern monitoring, privilege change alerts
   - **Response**: Automatic account suspension, incident creation

2. **Data Injection Attacks**
   - **Risk**: Malicious data corrupts system or exposes sensitive information
   - **Mitigation**: Input validation, parameterized queries, output encoding
   - **Detection**: Anomaly detection in event data, pattern matching
   - **Response**: Automatic data quarantine, security incident creation

3. **Session Hijacking**
   - **Risk**: Unauthorized access through compromised JWT tokens
   - **Mitigation**: Secure token storage, short expiration, token rotation
   - **Detection**: Unusual session patterns, concurrent session monitoring
   - **Response**: Token revocation, forced re-authentication

### Operational Risks

#### Business Continuity

1. **System Downtime**
   - **Risk**: Critical security monitoring unavailable during incidents
   - **Mitigation**: High availability deployment, health checks, automatic failover
   - **Recovery**: Documented recovery procedures, backup systems
   - **Communication**: Status page, alert notifications

2. **Data Loss**
   - **Risk**: Security events or incidents are lost due to system failure
   - **Mitigation**: Database replication, transaction logging, backup procedures
   - **Recovery**: Point-in-time recovery, data validation procedures
   - **Prevention**: Regular backup testing, disaster recovery drills

---

## Deployment Strategy

### Environment Configuration

#### Development Environment
```yaml
# docker-compose.dev.yml
version: '3.8'
services:
  postgres:
    image: postgres:14
    environment:
      POSTGRES_DB: oasis_dev
      POSTGRES_USER: dev_user
      POSTGRES_PASSWORD: dev_password
    volumes:
      - postgres_dev_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:6-alpine
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes
    volumes:
      - redis_dev_data:/data

  oasis_backend:
    build: .
    environment:
      DATABASE_URL: postgres://dev_user:dev_password@postgres:5432/oasis_dev
      REDIS_URL: redis://redis:6379
      RUST_LOG: debug
      JWT_SECRET: dev_jwt_secret_key_32_characters_min
    ports:
      - "8080:8080"
    depends_on:
      - postgres
      - redis
    volumes:
      - ./logs:/app/logs

volumes:
  postgres_dev_data:
  redis_dev_data:
```

#### Production Environment
```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  postgres:
    image: postgres:14
    environment:
      POSTGRES_DB: ${DB_NAME}
      POSTGRES_USER: ${DB_USER}
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_prod_data:/var/lib/postgresql/data
      - ./backups:/backups
    networks:
      - internal
    deploy:
      resources:
        limits:
          memory: 4G
          cpus: '2'

  redis:
    image: redis:6-alpine
    command: redis-server --appendonly yes --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis_prod_data:/data
    networks:
      - internal
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '0.5'

  oasis_backend:
    image: oasis:${VERSION}
    environment:
      DATABASE_URL: postgres://${DB_USER}:${DB_PASSWORD}@postgres:5432/${DB_NAME}
      REDIS_URL: redis://:${REDIS_PASSWORD}@redis:6379
      RUST_LOG: info
      JWT_SECRET: ${JWT_SECRET}
      ENCRYPTION_KEY: ${ENCRYPTION_KEY}
    networks:
      - internal
      - external
    deploy:
      replicas: 3
      resources:
        limits:
          memory: 2G
          cpus: '1'
      restart_policy:
        condition: any
        delay: 5s
        max_attempts: 3

  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/ssl/certs:ro
    networks:
      - external
    depends_on:
      - oasis_backend

networks:
  internal:
    driver: overlay
  external:
    driver: overlay
```

### Database Migration Strategy

#### Migration Management
```rust
// src/infrastructure/database/migration_manager.rs
pub struct MigrationManager {
    db_pool: Arc<PgPool>,
}

impl MigrationManager {
    pub async fn run_migrations(&self) -> Result<(), MigrationError> {
        info!("Starting database migrations...");
        
        // Check current migration version
        let current_version = self.get_current_migration_version().await?;
        info!("Current migration version: {}", current_version);
        
        // Get available migrations
        let available_migrations = self.get_available_migrations()?;
        let pending_migrations: Vec<_> = available_migrations
            .into_iter()
            .filter(|m| m.version > current_version)
            .collect();
            
        if pending_migrations.is_empty() {
            info!("No pending migrations");
            return Ok(());
        }
        
        info!("Found {} pending migrations", pending_migrations.len());
        
        // Run migrations in transaction
        let mut tx = self.db_pool.begin().await?;
        
        for migration in pending_migrations {
            info!("Applying migration: {}", migration.name);
            
            // Execute migration SQL
            sqlx::query(&migration.sql)
                .execute(&mut tx)
                .await
                .map_err(|e| MigrationError::ExecutionFailed(migration.name.clone(), e))?;
                
            // Update migration version
            sqlx::query!(
                "UPDATE schema_migrations SET version = $1, applied_at = now()",
                migration.version
            )
            .execute(&mut tx)
            .await?;
            
            info!("Migration {} applied successfully", migration.name);
        }
        
        tx.commit().await?;
        info!("All migrations completed successfully");
        
        Ok(())
    }
}
```

#### Deployment Pipeline
```bash
#!/bin/bash
# deploy.sh - Production deployment script

set -e

# Configuration
VERSION=$1
ENVIRONMENT=${2:-production}
BACKUP_RETENTION_DAYS=30

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version> [environment]"
    exit 1
fi

echo "Starting deployment of version $VERSION to $ENVIRONMENT"

# Pre-deployment checks
echo "Running pre-deployment checks..."
./scripts/check-database-health.sh
./scripts/check-redis-health.sh
./scripts/validate-configuration.sh $ENVIRONMENT

# Create database backup
echo "Creating database backup..."
BACKUP_FILE="backup-$(date +%Y%m%d_%H%M%S)-pre-$VERSION.sql"
docker exec $(docker ps -q --filter name=postgres) pg_dump -U $DB_USER $DB_NAME > backups/$BACKUP_FILE
echo "Backup created: $BACKUP_FILE"

# Build and push new image
echo "Building application image..."
docker build -t oasis:$VERSION .
docker tag oasis:$VERSION registry.company.com/oasis:$VERSION
docker push registry.company.com/oasis:$VERSION

# Run database migrations
echo "Running database migrations..."
docker run --rm --network oasis_internal \
    -e DATABASE_URL="postgres://$DB_USER:$DB_PASSWORD@postgres:5432/$DB_NAME" \
    oasis:$VERSION migrate

# Rolling deployment
echo "Starting rolling deployment..."
docker service update --image registry.company.com/oasis:$VERSION oasis_backend

# Health check
echo "Waiting for deployment to stabilize..."
sleep 30

for i in {1..12}; do
    if curl -f http://localhost/health; then
        echo "Deployment successful!"
        break
    fi
    
    if [ $i -eq 12 ]; then
        echo "Deployment failed - health check timeout"
        echo "Rolling back..."
        ./scripts/rollback.sh
        exit 1
    fi
    
    echo "Health check failed, retrying in 10 seconds..."
    sleep 10
done

# Cleanup old backups
find backups/ -name "*.sql" -mtime +$BACKUP_RETENTION_DAYS -delete

echo "Deployment completed successfully!"
```

### Monitoring and Observability

#### Metrics Collection
```rust
// src/infrastructure/monitoring/metrics.rs
use prometheus::{Counter, Histogram, Gauge, Registry};

pub struct SecurityMetrics {
    // Event processing metrics
    events_processed_total: Counter,
    event_processing_duration: Histogram,
    events_buffer_size: Gauge,
    
    // Correlation metrics
    correlations_created_total: Counter,
    correlation_confidence_distribution: Histogram,
    
    // Incident metrics
    incidents_created_total: Counter,
    incidents_resolved_total: Counter,
    incident_resolution_time: Histogram,
    
    // System metrics
    websocket_connections_active: Gauge,
    database_queries_duration: Histogram,
    cache_hit_ratio: Gauge,
}

impl SecurityMetrics {
    pub fn new(registry: &Registry) -> Self {
        let events_processed_total = Counter::new(
            "security_events_processed_total",
            "Total number of security events processed"
        ).unwrap();
        registry.register(Box::new(events_processed_total.clone())).unwrap();
        
        let event_processing_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "security_event_processing_duration_seconds",
                "Time spent processing security events"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
        ).unwrap();
        registry.register(Box::new(event_processing_duration.clone())).unwrap();
        
        // ... register other metrics
        
        Self {
            events_processed_total,
            event_processing_duration,
            // ... other metrics
        }
    }
    
    pub fn record_event_processed(&self, processing_time: Duration) {
        self.events_processed_total.inc();
        self.event_processing_duration.observe(processing_time.as_secs_f64());
    }
}
```

#### Health Check Endpoints
```rust
// src/api/routes/health.rs
#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    database: DatabaseHealth,
    cache: CacheHealth,
    processing: ProcessingHealth,
    timestamp: DateTime<Utc>,
}

#[get("/health")]
pub async fn health_check(
    db_pool: web::Data<PgPool>,
    redis_pool: web::Data<RedisPool>,
    metrics: web::Data<SecurityMetrics>,
) -> Result<HttpResponse, ApiError> {
    let database_health = check_database_health(&db_pool).await?;
    let cache_health = check_cache_health(&redis_pool).await?;
    let processing_health = check_processing_health(&metrics).await?;
    
    let overall_status = if database_health.healthy && cache_health.healthy && processing_health.healthy {
        "healthy"
    } else {
        "unhealthy"
    };
    
    let response = HealthResponse {
        status: overall_status.to_string(),
        database: database_health,
        cache: cache_health,
        processing: processing_health,
        timestamp: Utc::now(),
    };
    
    let status_code = if overall_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    
    Ok(HttpResponse::build(status_code).json(response))
}
```

---

## Conclusion

This enhanced security implementation plan provides a comprehensive, realistic approach to transforming the existing placeholder security incidents system into a fully functional security event monitoring and incident management platform. 

### Key Improvements Made

1. **Fixed Critical Database Issues**: Resolved schema ordering conflicts and added comprehensive indexing
2. **Realistic Timeline**: Adjusted from 8-12 weeks to 16-20 weeks with detailed task breakdown
3. **Complete Technical Specifications**: Added detailed real-time processing and correlation engine architecture
4. **Enhanced Security**: Comprehensive access control, input validation, and data protection measures
5. **Performance Optimization**: Detailed caching strategy, connection pooling, and query optimization
6. **Comprehensive Testing**: Unit, integration, performance, and security testing strategies
7. **Deployment Strategy**: Production-ready deployment with monitoring and rollback procedures

### Success Factors

- **Incremental Development**: Phased approach allows for early feedback and course correction
- **Comprehensive Testing**: Multiple testing layers ensure system reliability and performance
- **Security-First Design**: Built-in security considerations at every layer
- **Realistic Resource Planning**: Proper allocation of development resources and expertise
- **Performance Monitoring**: Comprehensive metrics and alerting for operational excellence

This implementation plan provides a solid foundation for building a production-ready security monitoring system that can scale to handle enterprise-level security event processing and incident management requirements.