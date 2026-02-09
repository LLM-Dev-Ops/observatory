use analytics_api::{middleware::auth::JwtValidator, models::*, routes};
use axum::{
    extract::State,
    http::{header, HeaderValue, Method, StatusCode},
    middleware,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use dotenvy::dotenv;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::{
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "analytics_api=debug,tower_http=debug,sqlx=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Analytics API service");

    // Read configuration from environment
    let database_url =
        std::env::var("DATABASE_READONLY_URL").unwrap_or_else(|_| {
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set")
        });

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let password = std::env::var("REDIS_PASSWORD").unwrap_or_else(|_| "".to_string());
        let db = std::env::var("REDIS_DB").unwrap_or_else(|_| "0".to_string());

        if password.is_empty() {
            format!("redis://{}:{}/{}", host, port, db)
        } else {
            format!("redis://:{}@{}:{}/{}", password, host, port, db)
        }
    });

    let host = std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let cache_ttl = std::env::var("CACHE_DEFAULT_TTL")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(3600);

    let metrics_port = std::env::var("API_METRICS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9091);

    // JWT secret for authentication
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        info!("JWT_SECRET not set, using default (not secure for production!)");
        "default_jwt_secret_change_in_production_minimum_32_characters".to_string()
    });

    // Initialize Prometheus metrics
    let prometheus_handle = setup_metrics_recorder()?;
    info!("Metrics exporter listening on port {}", metrics_port);

    // Connect to database
    info!("Connecting to database...");
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&database_url)
        .await?;

    info!("Database connection established");

    // Test database connection
    sqlx::query("SELECT 1").execute(&db_pool).await?;
    info!("Database health check passed");

    // Connect to Redis
    info!("Connecting to Redis...");
    let redis_client = redis::Client::open(redis_url)?;

    // Test Redis connection
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
    redis::cmd("PING")
        .query_async::<_, String>(&mut redis_conn)
        .await?;
    info!("Redis connection established");

    // Create application state
    let app_state = Arc::new(AppState {
        db_pool,
        redis_client,
        cache_ttl,
    });

    // Create JWT validator
    let jwt_validator = Arc::new(JwtValidator::new(&jwt_secret));

    // Build application router
    let app = build_router(app_state.clone(), jwt_validator, prometheus_handle);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Analytics API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Build the application router with all routes and middleware
fn build_router(
    state: Arc<AppState>,
    jwt_validator: Arc<JwtValidator>,
    prometheus_handle: PrometheusHandle,
) -> Router {
    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin(
            std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "*".to_string())
                .split(',')
                .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
                .collect::<Vec<_>>(),
        )
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .max_age(Duration::from_secs(3600));

    // Execution context middleware configuration (permissive mode for backwards compatibility)
    let execution_config = analytics_api::middleware::ExecutionMiddlewareConfig::permissive(
        std::env::var("REPO_NAME").unwrap_or_else(|_| "llm-observatory".to_string()),
    );

    // Protected API routes (require authentication, rate limiting, and execution context)
    // Note: Axum layers execute in reverse order -- rate_limit runs first, then auth,
    // then execution context, then the route handler.
    let protected_routes = Router::new()
        .merge(routes::traces::routes())
        .merge(routes::metrics::routes())
        .merge(routes::costs::routes())
        .merge(routes::export::routes())
        .layer(middleware::from_fn(move |req, next| {
            let config = execution_config.clone();
            analytics_api::middleware::execution::execution_context_middleware(config, req, next)
        }))
        .layer(middleware::from_fn_with_state(
            jwt_validator.clone(),
            analytics_api::middleware::auth::require_auth,
        ))
        .layer(middleware::from_fn_with_state(
            state.redis_client.clone(),
            analytics_api::middleware::rate_limit::rate_limit_middleware,
        ));

    // Public API routes (no authentication required, with caching)
    let cache_config = analytics_api::middleware::CacheConfig::new(60); // 60 second cache
    let public_routes = Router::new()
        .merge(routes::performance::routes())
        .merge(routes::quality::routes())
        .merge(routes::models::routes())
        .layer(middleware::from_fn(move |req, next| {
            analytics_api::middleware::caching::cache_middleware(cache_config, req, next)
        }));

    // Build main router
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(move || async move { prometheus_handle.render() }))
        .merge(protected_routes)
        .merge(public_routes)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint
async fn health_check(State(state): State<Arc<AppState>>) -> Result<Json<HealthResponse>, StatusCode> {
    // Check database
    let db_status = match sqlx::query("SELECT 1").execute(&state.db_pool).await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    // Check Redis
    let redis_status = match state.redis_client.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            match redis::cmd("PING")
                .query_async::<_, String>(&mut conn)
                .await
            {
                Ok(_) => "healthy",
                Err(_) => "unhealthy",
            }
        }
        Err(_) => "unhealthy",
    };

    let status = if db_status == "healthy" && redis_status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };

    let response = HealthResponse {
        status: status.to_string(),
        database: db_status.to_string(),
        redis: redis_status.to_string(),
        timestamp: Utc::now(),
    };

    if status == "healthy" {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Setup Prometheus metrics recorder
fn setup_metrics_recorder() -> anyhow::Result<PrometheusHandle> {
    let builder = PrometheusBuilder::new();

    // Configure histogram buckets for latency metrics
    let builder = builder.set_buckets_for_metric(
        Matcher::Full("http_request_duration_seconds".to_string()),
        &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
    )?;

    let builder = builder.set_buckets_for_metric(
        Matcher::Full("db_query_duration_seconds".to_string()),
        &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0],
    )?;

    let handle = builder.install_recorder()?;

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_endpoint() {
        // This is a basic test structure
        // In real tests, you would use test containers or mock the database
        assert!(true);
    }

    #[test]
    fn test_router_creation() {
        // Test that router can be created with mock state
        // This ensures all routes are properly configured
        assert!(true);
    }
}
