//! Test connection binary for storage layer.
//!
//! This binary tests the storage configuration and connection pooling,
//! verifying connectivity to PostgreSQL and Redis (if configured).
//!
//! # Usage
//!
//! ```bash
//! # Using environment variables
//! export DB_PASSWORD=mypassword
//! cargo run --bin test_connection
//!
//! # Using .env file
//! cargo run --bin test_connection
//!
//! # With verbose logging
//! RUST_LOG=debug cargo run --bin test_connection
//! ```

use llm_observatory_storage::{StorageConfig, StoragePool};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("\n========================================");
    println!("  LLM Observatory Storage Test");
    println!("========================================\n");

    // Step 1: Load configuration
    println!("Step 1: Loading configuration from environment...");
    let config = match StorageConfig::from_env() {
        Ok(config) => {
            println!("✓ Configuration loaded successfully");
            println!(
                "  - PostgreSQL: {}:{}/{}",
                config.postgres.host, config.postgres.port, config.postgres.database
            );
            println!(
                "  - Redis: {}",
                if config.redis.is_some() {
                    "Configured"
                } else {
                    "Not configured"
                }
            );
            println!("  - Pool Max Connections: {}", config.pool.max_connections);
            println!("  - Pool Min Connections: {}", config.pool.min_connections);
            println!("  - Retry Max Attempts: {}", config.retry.max_retries);
            config
        }
        Err(e) => {
            eprintln!("✗ Failed to load configuration: {}", e);
            eprintln!("\nMake sure to set required environment variables:");
            eprintln!("  - DB_PASSWORD (required)");
            eprintln!("  - Or use a .env file in the project root");
            std::process::exit(1);
        }
    };

    println!();

    // Step 2: Validate configuration
    println!("Step 2: Validating configuration...");
    if let Err(e) = config.validate() {
        eprintln!("✗ Configuration validation failed: {}", e);
        std::process::exit(1);
    }
    println!("✓ Configuration is valid");
    println!();

    // Step 3: Create connection pool
    println!("Step 3: Creating connection pool...");
    let pool = match StoragePool::new(config).await {
        Ok(pool) => {
            println!("✓ Connection pool created successfully");
            pool
        }
        Err(e) => {
            eprintln!("✗ Failed to create connection pool: {}", e);
            eprintln!("\nPossible issues:");
            eprintln!("  - PostgreSQL is not running");
            eprintln!("  - Credentials are incorrect");
            eprintln!("  - Network connectivity issues");
            std::process::exit(1);
        }
    };

    println!();

    // Step 4: Test PostgreSQL connection
    println!("Step 4: Testing PostgreSQL connection...");
    match pool.health_check_postgres().await {
        Ok(_) => println!("✓ PostgreSQL connection successful"),
        Err(e) => {
            eprintln!("✗ PostgreSQL health check failed: {}", e);
            std::process::exit(1);
        }
    }

    println!();

    // Step 5: Test Redis connection (if configured)
    println!("Step 5: Testing Redis connection...");
    match pool.health_check_redis().await {
        Ok(_) => println!("✓ Redis connection successful"),
        Err(e) => {
            if pool.redis().is_some() {
                eprintln!("⚠ Redis health check failed: {}", e);
                eprintln!("  (This is non-fatal, continuing without Redis)");
            } else {
                println!("⊘ Redis not configured (skipping)");
            }
        }
    }

    println!();

    // Step 6: Run comprehensive health check
    println!("Step 6: Running comprehensive health check...");
    match pool.health_check().await {
        Ok(result) => {
            println!("✓ Health check completed");
            println!(
                "  - PostgreSQL: {}",
                if result.postgres_healthy {
                    "Healthy"
                } else {
                    "Unhealthy"
                }
            );
            println!(
                "  - Redis: {}",
                match result.redis_healthy {
                    Some(true) => "Healthy",
                    Some(false) => "Unhealthy",
                    None => "Not configured",
                }
            );
            println!(
                "  - Overall: {}",
                if result.is_healthy() {
                    "Healthy"
                } else {
                    "Degraded"
                }
            );
        }
        Err(e) => {
            eprintln!("✗ Health check failed: {}", e);
            std::process::exit(1);
        }
    }

    println!();

    // Step 7: Get and display pool statistics
    println!("Step 7: Gathering pool statistics...");
    let stats = pool.stats();
    println!("✓ Pool statistics:");
    println!("  - Total connections: {}", stats.postgres_size);
    println!("  - Active connections: {}", stats.postgres_active);
    println!("  - Idle connections: {}", stats.postgres_idle);
    println!("  - Max connections: {}", stats.postgres_max_connections);
    println!("  - Min connections: {}", stats.postgres_min_connections);
    println!("  - Utilization: {:.1}%", stats.utilization_percent());
    println!(
        "  - Near capacity: {}",
        if stats.is_near_capacity() {
            "Yes (>80%)"
        } else {
            "No"
        }
    );
    println!("  - Redis connected: {}", stats.redis_connected);

    println!();

    // Step 8: Test a simple query
    println!("Step 8: Testing simple query...");
    match sqlx::query("SELECT NOW() as current_time, version() as pg_version")
        .fetch_one(pool.postgres())
        .await
    {
        Ok(row) => {
            let current_time: chrono::DateTime<chrono::Utc> = row.get("current_time");
            let pg_version: String = row.get("pg_version");
            println!("✓ Query executed successfully");
            println!("  - Current time: {}", current_time);
            println!("  - PostgreSQL version: {}", pg_version);
        }
        Err(e) => {
            eprintln!("✗ Query failed: {}", e);
            std::process::exit(1);
        }
    }

    println!();

    // Step 9: Close connections
    println!("Step 9: Closing connections...");
    pool.close().await;
    println!("✓ Connections closed gracefully");

    println!();
    println!("========================================");
    println!("  All tests passed! ✓");
    println!("========================================\n");
    println!("Your storage configuration is working correctly.");
    println!("You can now use the storage pool in your application.\n");
}
