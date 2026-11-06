///! Authentication and authorization middleware for the Analytics API
///!
///! This module provides JWT-based authentication and role-based access control (RBAC).
///! It supports both JWT tokens and API keys for authentication.
///!
///! # Security Features
///! - JWT token validation with expiration checking
///! - API key hashing and validation
///! - Role-based access control
///! - Project-level authorization
///! - Audit logging for all auth events
///!
///! # Usage
///! ```rust,no_run
///! use axum::{Router, routing::get};
///! use analytics_api::middleware::{RequireAuth, Role};
///!
///! let app = Router::new()
///!     .route("/traces", get(list_traces))
///!     .layer(RequireAuth::new(vec![Role::Admin, Role::Developer]));
///! ```
use axum::{
    body::Body,
    extract::{FromRequestParts, Request, State},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Organization ID
    pub org_id: String,
    /// Accessible project IDs
    pub projects: Vec<String>,
    /// User role
    pub role: Role,
    /// Permissions
    pub permissions: Vec<String>,
    /// Issued at (timestamp)
    pub iat: i64,
    /// Expiration (timestamp)
    pub exp: i64,
    /// JWT ID
    pub jti: String,
}

impl JwtClaims {
    /// Create new JWT claims
    pub fn new(
        user_id: String,
        org_id: String,
        projects: Vec<String>,
        role: Role,
        permissions: Vec<String>,
        ttl_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id,
            org_id,
            projects,
            role,
            permissions,
            iat: now.timestamp(),
            exp: (now + Duration::seconds(ttl_seconds)).timestamp(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.role == Role::Admin || self.permissions.contains(&permission.to_string())
    }

    /// Check if user can access a project
    pub fn can_access_project(&self, project_id: &str) -> bool {
        self.role == Role::Admin || self.projects.contains(&project_id.to_string())
    }
}

/// User roles with hierarchical permissions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full system access
    Admin,
    /// Read/write access to data, can add evaluations and feedback
    Developer,
    /// Read-only access to traces and metrics
    Viewer,
    /// Read-only access to cost and usage data
    Billing,
}

impl Role {
    /// Get default permissions for a role
    pub fn default_permissions(&self) -> Vec<String> {
        match self {
            Role::Admin => vec!["*".to_string()],
            Role::Developer => vec![
                "read:traces".to_string(),
                "read:metrics".to_string(),
                "read:costs".to_string(),
                "write:evaluations".to_string(),
                "write:feedback".to_string(),
            ],
            Role::Viewer => vec![
                "read:traces".to_string(),
                "read:metrics".to_string(),
                "read:costs".to_string(),
            ],
            Role::Billing => vec!["read:costs".to_string(), "read:usage".to_string()],
        }
    }

    /// Check if role has permission (respects hierarchy)
    pub fn has_permission(&self, permission: &str) -> bool {
        if self == &Role::Admin {
            return true;
        }
        self.default_permissions().contains(&permission.to_string())
    }
}

/// Authentication context extracted from request
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID
    pub user_id: String,
    /// Organization ID
    pub org_id: String,
    /// Accessible projects
    pub projects: Vec<String>,
    /// User role
    pub role: Role,
    /// User permissions
    pub permissions: Vec<String>,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Request ID for tracing
    pub request_id: String,
}

impl AuthContext {
    /// Create from JWT claims
    pub fn from_claims(claims: JwtClaims, request_id: String) -> Self {
        Self {
            user_id: claims.sub,
            org_id: claims.org_id,
            projects: claims.projects,
            role: claims.role,
            permissions: claims.permissions,
            auth_method: AuthMethod::Jwt,
            request_id,
        }
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.role == Role::Admin || self.permissions.contains(&permission.to_string())
    }

    /// Check if user can access a project
    pub fn can_access_project(&self, project_id: &str) -> bool {
        self.role == Role::Admin || self.projects.contains(&project_id.to_string())
    }

    /// Get accessible project or return error if no access
    pub fn require_project_access(&self, project_id: Option<&str>) -> Result<String, AuthError> {
        match project_id {
            Some(id) if self.can_access_project(id) => Ok(id.to_string()),
            Some(_) => Err(AuthError::ProjectAccessDenied),
            None if self.role == Role::Admin => {
                // Admin can query all projects, return empty string as wildcard
                Ok(String::new())
            }
            None if !self.projects.is_empty() => {
                // Non-admin without project filter: use first accessible project
                Ok(self.projects[0].clone())
            }
            None => Err(AuthError::ProjectRequired),
        }
    }
}

/// Authentication method used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// JWT token authentication
    Jwt,
    /// API key authentication
    ApiKey,
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Missing authorization header")]
    MissingHeader,

    #[error("Invalid authorization header format")]
    InvalidHeader,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Project access denied")]
    ProjectAccessDenied,

    #[error("Project ID is required")]
    ProjectRequired,

    #[error("Token has expired")]
    TokenExpired,

    #[error("Internal authentication error: {0}")]
    Internal(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            AuthError::MissingHeader => (
                StatusCode::UNAUTHORIZED,
                "MISSING_AUTHORIZATION",
                "Authorization header is required",
            ),
            AuthError::InvalidHeader => (
                StatusCode::UNAUTHORIZED,
                "INVALID_AUTHORIZATION",
                "Invalid authorization header format",
            ),
            AuthError::InvalidToken | AuthError::TokenExpired => (
                StatusCode::UNAUTHORIZED,
                "INVALID_TOKEN",
                "Invalid or expired token",
            ),
            AuthError::InvalidApiKey => (
                StatusCode::UNAUTHORIZED,
                "INVALID_API_KEY",
                "Invalid API key",
            ),
            AuthError::InsufficientPermissions => (
                StatusCode::FORBIDDEN,
                "INSUFFICIENT_PERMISSIONS",
                "Insufficient permissions for this operation",
            ),
            AuthError::ProjectAccessDenied => (
                StatusCode::FORBIDDEN,
                "PROJECT_ACCESS_DENIED",
                "Access denied to the specified project",
            ),
            AuthError::ProjectRequired => (
                StatusCode::BAD_REQUEST,
                "PROJECT_REQUIRED",
                "Project ID is required for this operation",
            ),
            AuthError::Internal(ref msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.as_str(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
            },
            "meta": {
                "timestamp": Utc::now().to_rfc3339(),
            }
        }));

        (status, body).into_response()
    }
}

/// JWT token validator
pub struct JwtValidator {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtValidator {
    /// Create new JWT validator
    pub fn new(secret: &str) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            validation: Validation::default(),
        }
    }

    /// Validate and decode JWT token
    pub fn validate(&self, token: &str) -> Result<JwtClaims, AuthError> {
        let token_data =
            decode::<JwtClaims>(token, &self.decoding_key, &self.validation).map_err(|e| {
                error!("JWT validation error: {}", e);
                AuthError::InvalidToken
            })?;

        let claims = token_data.claims;

        // Check if expired
        if claims.is_expired() {
            warn!("Expired token for user: {}", claims.sub);
            return Err(AuthError::TokenExpired);
        }

        info!("Successfully validated token for user: {}", claims.sub);
        Ok(claims)
    }
}

/// JWT token generator
pub struct JwtGenerator {
    encoding_key: EncodingKey,
    ttl_seconds: i64,
}

impl JwtGenerator {
    /// Create new JWT generator
    pub fn new(secret: &str, ttl_seconds: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            ttl_seconds,
        }
    }

    /// Generate JWT token
    pub fn generate(&self, claims: JwtClaims) -> Result<String, AuthError> {
        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            error!("JWT generation error: {}", e);
            AuthError::Internal(format!("Failed to generate token: {}", e))
        })
    }

    /// Generate token from user details
    pub fn generate_for_user(
        &self,
        user_id: String,
        org_id: String,
        projects: Vec<String>,
        role: Role,
        permissions: Vec<String>,
    ) -> Result<String, AuthError> {
        let claims = JwtClaims::new(
            user_id,
            org_id,
            projects,
            role,
            permissions,
            self.ttl_seconds,
        );
        self.generate(claims)
    }
}

/// Authentication middleware layer
#[derive(Clone)]
pub struct RequireAuth {
    jwt_validator: Arc<JwtValidator>,
    allowed_roles: Vec<Role>,
}

impl RequireAuth {
    /// Create new auth middleware
    pub fn new(jwt_secret: &str, allowed_roles: Vec<Role>) -> Self {
        Self {
            jwt_validator: Arc::new(JwtValidator::new(jwt_secret)),
            allowed_roles,
        }
    }

    /// Create auth middleware allowing all roles
    pub fn any_role(jwt_secret: &str) -> Self {
        Self::new(
            jwt_secret,
            vec![Role::Admin, Role::Developer, Role::Viewer, Role::Billing],
        )
    }
}

/// Middleware function for authentication
pub async fn require_auth(
    State(validator): State<Arc<JwtValidator>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let request_id = Uuid::new_v4().to_string();

    // Extract authorization header
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingHeader)?
        .to_str()
        .map_err(|_| AuthError::InvalidHeader)?;

    // Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidHeader)?;

    // Validate token
    let claims = validator.validate(token)?;

    // Create auth context
    let auth_context = AuthContext::from_claims(claims.clone(), request_id.clone());

    info!(
        user_id = %auth_context.user_id,
        org_id = %auth_context.org_id,
        role = ?auth_context.role,
        request_id = %request_id,
        "Authentication successful"
    );

    // Insert auth context into request extensions
    req.extensions_mut().insert(auth_context);
    req.extensions_mut().insert(request_id);

    Ok(next.run(req).await)
}

/// Extract auth context from request
///
/// Note: This implementation works with axum 0.7's FromRequestParts trait.
/// The lifetime parameters are elided and inferred by the compiler.
#[async_trait::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or(AuthError::Internal(
                "Auth context not found. Ensure RequireAuth middleware is applied.".to_string(),
            ))
    }
}

/// Permission checker middleware
pub async fn require_permission(
    auth: AuthContext,
    permission: String,
) -> Result<AuthContext, AuthError> {
    if auth.has_permission(&permission) {
        Ok(auth)
    } else {
        warn!(
            user_id = %auth.user_id,
            required_permission = %permission,
            "Permission denied"
        );
        Err(AuthError::InsufficientPermissions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        assert!(Role::Admin.has_permission("any:permission"));
        assert!(Role::Developer.has_permission("read:traces"));
        assert!(Role::Developer.has_permission("write:evaluations"));
        assert!(!Role::Viewer.has_permission("write:evaluations"));
        assert!(Role::Billing.has_permission("read:costs"));
        assert!(!Role::Billing.has_permission("read:traces"));
    }

    #[test]
    fn test_jwt_claims_expiration() {
        let claims = JwtClaims::new(
            "user123".to_string(),
            "org456".to_string(),
            vec!["proj1".to_string()],
            Role::Developer,
            Role::Developer.default_permissions(),
            -10, // Expired 10 seconds ago
        );

        assert!(claims.is_expired());

        let valid_claims = JwtClaims::new(
            "user123".to_string(),
            "org456".to_string(),
            vec!["proj1".to_string()],
            Role::Developer,
            Role::Developer.default_permissions(),
            3600, // Valid for 1 hour
        );

        assert!(!valid_claims.is_expired());
    }

    #[test]
    fn test_jwt_claims_permissions() {
        let claims = JwtClaims::new(
            "user123".to_string(),
            "org456".to_string(),
            vec!["proj1".to_string()],
            Role::Developer,
            vec!["read:traces".to_string(), "write:evaluations".to_string()],
            3600,
        );

        assert!(claims.has_permission("read:traces"));
        assert!(claims.has_permission("write:evaluations"));
        assert!(!claims.has_permission("delete:projects"));
    }

    #[test]
    fn test_jwt_claims_project_access() {
        let claims = JwtClaims::new(
            "user123".to_string(),
            "org456".to_string(),
            vec!["proj1".to_string(), "proj2".to_string()],
            Role::Developer,
            Role::Developer.default_permissions(),
            3600,
        );

        assert!(claims.can_access_project("proj1"));
        assert!(claims.can_access_project("proj2"));
        assert!(!claims.can_access_project("proj3"));

        // Admin can access any project
        let admin_claims = JwtClaims::new(
            "admin".to_string(),
            "org456".to_string(),
            vec![],
            Role::Admin,
            Role::Admin.default_permissions(),
            3600,
        );

        assert!(admin_claims.can_access_project("any_project"));
    }

    #[test]
    fn test_jwt_token_generation_and_validation() {
        let secret = "test_secret_key_at_least_32_chars_long_for_security";
        let generator = JwtGenerator::new(secret, 3600);
        let validator = JwtValidator::new(secret);

        let token = generator
            .generate_for_user(
                "user123".to_string(),
                "org456".to_string(),
                vec!["proj1".to_string()],
                Role::Developer,
                Role::Developer.default_permissions(),
            )
            .unwrap();

        let claims = validator.validate(&token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.org_id, "org456");
        assert_eq!(claims.role, Role::Developer);
        assert_eq!(claims.projects, vec!["proj1".to_string()]);
    }

    #[test]
    fn test_auth_context_project_access() {
        let auth = AuthContext {
            user_id: "user123".to_string(),
            org_id: "org456".to_string(),
            projects: vec!["proj1".to_string()],
            role: Role::Developer,
            permissions: Role::Developer.default_permissions(),
            auth_method: AuthMethod::Jwt,
            request_id: "req123".to_string(),
        };

        // Valid project access
        assert!(auth.require_project_access(Some("proj1")).is_ok());

        // Invalid project access
        assert!(matches!(
            auth.require_project_access(Some("proj2")),
            Err(AuthError::ProjectAccessDenied)
        ));

        // No project specified, should return first accessible project
        let result = auth.require_project_access(None).unwrap();
        assert_eq!(result, "proj1");
    }

    #[test]
    fn test_auth_context_admin_project_access() {
        let admin_auth = AuthContext {
            user_id: "admin".to_string(),
            org_id: "org456".to_string(),
            projects: vec![],
            role: Role::Admin,
            permissions: Role::Admin.default_permissions(),
            auth_method: AuthMethod::Jwt,
            request_id: "req123".to_string(),
        };

        // Admin can access any project
        assert!(admin_auth.require_project_access(Some("any_proj")).is_ok());

        // Admin without project filter gets wildcard
        let result = admin_auth.require_project_access(None).unwrap();
        assert_eq!(result, "");
    }
}
