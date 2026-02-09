pub mod errors;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;

// Re-export commonly used types
pub use errors::{ApiError, ErrorCategory, ErrorCode};
pub use middleware::{AuthContext, JwtClaims, RequireAuth, Role};
pub use middleware::{ExecutionMiddlewareConfig, ReqExecutionContext, execution_context_middleware};
pub use models::{AppState, AnalyticsQuery, ErrorResponse, HealthResponse};
pub use services::timescaledb::TimescaleDBService;
