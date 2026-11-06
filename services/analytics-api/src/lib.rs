pub mod errors;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;

// Re-export commonly used types
pub use errors::{ApiError, ErrorCategory, ErrorCode};
pub use middleware::{
    execution_context_middleware, ExecutionMiddlewareConfig, ReqExecutionContext,
};
pub use middleware::{AuthContext, JwtClaims, RequireAuth, Role};
pub use models::{AnalyticsQuery, AppState, ErrorResponse, HealthResponse};
pub use services::timescaledb::TimescaleDBService;
