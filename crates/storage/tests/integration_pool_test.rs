//! Integration tests for connection pool management.
//!
//! This test suite validates connection pool creation, health checks,
//! statistics, and connection lifecycle management.

mod common;

use common::*;
use llm_observatory_storage::{StorageConfig, StoragePool};

#[tokio::test]
async fn test_pool_creation() {
    let (pool, _guard) = setup_test_pool().await;

    // Verify pool was created
    assert!(pool.postgres().size() > 0);
}

#[tokio::test]
async fn test_pool_postgres_health_check() {
    let (pool, _guard) = setup_test_pool().await;

    // Run health check
    let result = pool.health_check_postgres().await;
    assert!(result.is_ok(), "PostgreSQL health check should pass");
}

#[tokio::test]
async fn test_pool_full_health_check() {
    let (pool, _guard) = setup_test_pool().await;

    // Run full health check
    let result = pool.health_check().await;
    assert!(result.is_ok(), "Health check should pass");

    let health = result.unwrap();
    assert!(health.postgres_healthy, "PostgreSQL should be healthy");
    assert!(health.is_healthy(), "Overall health should be true");
}

#[tokio::test]
async fn test_pool_health_check_without_redis() {
    let (pool, _guard) = setup_test_pool().await;

    let health = pool.health_check().await.unwrap();
    assert!(health.postgres_healthy);
    assert_eq!(health.redis_healthy, None, "Redis should not be configured");
}

#[tokio::test]
async fn test_pool_stats() {
    let (pool, _guard) = setup_test_pool().await;

    let stats = pool.stats();

    assert!(stats.postgres_size > 0, "Pool should have connections");
    assert_eq!(stats.postgres_max_connections, 10);
    assert_eq!(stats.postgres_min_connections, 1);
    assert!(!stats.redis_connected, "Redis should not be connected");
}

#[tokio::test]
async fn test_pool_stats_utilization() {
    let (pool, _guard) = setup_test_pool().await;

    let stats = pool.stats();
    let utilization = stats.utilization_percent();

    assert!(
        utilization >= 0.0 && utilization <= 100.0,
        "Utilization should be a valid percentage"
    );
}

#[tokio::test]
async fn test_pool_near_capacity_check() {
    let (pool, _guard) = setup_test_pool().await;

    let stats = pool.stats();

    // With default setup, we shouldn't be near capacity
    assert!(
        !stats.is_near_capacity(),
        "Pool should not be near capacity with default settings"
    );
}

#[tokio::test]
async fn test_pool_concurrent_connections() {
    let (pool, _guard) = setup_test_pool().await;

    // Execute multiple queries concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                let result: Result<(i32,), sqlx::Error> = sqlx::query_as("SELECT $1::int")
                    .bind(i)
                    .fetch_one(pool_clone.postgres())
                    .await;
                result.unwrap()
            })
        })
        .collect();

    // Wait for all queries to complete
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        assert_eq!(result.0, i as i32);
    }
}

#[tokio::test]
async fn test_pool_query_execution() {
    let (pool, _guard) = setup_test_pool().await;

    // Execute a simple query
    let result: (i32,) = sqlx::query_as("SELECT 1 + 1")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 2);
}

#[tokio::test]
async fn test_pool_multiple_queries() {
    let (pool, _guard) = setup_test_pool().await;

    // Execute multiple queries sequentially
    for i in 0..10 {
        let result: (i32,) = sqlx::query_as("SELECT $1::int")
            .bind(i)
            .fetch_one(pool.postgres())
            .await
            .unwrap();

        assert_eq!(result.0, i);
    }
}

#[tokio::test]
async fn test_pool_connection_reuse() {
    let (pool, _guard) = setup_test_pool().await;

    let initial_stats = pool.stats();
    let initial_size = initial_stats.postgres_size;

    // Execute several queries
    for _ in 0..5 {
        let _: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(pool.postgres())
            .await
            .unwrap();
    }

    let final_stats = pool.stats();

    // Pool size should remain relatively stable (connections are reused)
    assert!(
        final_stats.postgres_size <= initial_size + 2,
        "Pool should reuse connections efficiently"
    );
}

#[tokio::test]
async fn test_pool_config_access() {
    let (pool, _guard) = setup_test_pool().await;

    let config = pool.config();
    assert_eq!(config.pool.max_connections, 10);
    assert_eq!(config.pool.min_connections, 1);
}

#[tokio::test]
async fn test_pool_transaction_support() {
    let (pool, _guard) = setup_test_pool().await;

    // Create a temporary table
    sqlx::query("CREATE TEMPORARY TABLE test_table (id INTEGER, value TEXT)")
        .execute(pool.postgres())
        .await
        .unwrap();

    // Start a transaction
    let mut tx = pool.postgres().begin().await.unwrap();

    // Insert data within transaction
    sqlx::query("INSERT INTO test_table (id, value) VALUES ($1, $2)")
        .bind(1)
        .bind("test")
        .execute(&mut *tx)
        .await
        .unwrap();

    // Commit transaction
    tx.commit().await.unwrap();

    // Verify data was committed
    let result: (i32, String) = sqlx::query_as("SELECT id, value FROM test_table WHERE id = 1")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, "test");
}

#[tokio::test]
async fn test_pool_transaction_rollback() {
    let (pool, _guard) = setup_test_pool().await;

    // Create a temporary table
    sqlx::query("CREATE TEMPORARY TABLE test_rollback (id INTEGER, value TEXT)")
        .execute(pool.postgres())
        .await
        .unwrap();

    // Start a transaction
    let mut tx = pool.postgres().begin().await.unwrap();

    // Insert data within transaction
    sqlx::query("INSERT INTO test_rollback (id, value) VALUES ($1, $2)")
        .bind(1)
        .bind("test")
        .execute(&mut *tx)
        .await
        .unwrap();

    // Rollback transaction
    tx.rollback().await.unwrap();

    // Verify data was not committed
    let result: Option<(i32,)> = sqlx::query_as("SELECT id FROM test_rollback WHERE id = 1")
        .fetch_optional(pool.postgres())
        .await
        .unwrap();

    assert!(result.is_none(), "Data should not exist after rollback");
}

#[tokio::test]
async fn test_pool_prepared_statements() {
    let (pool, _guard) = setup_test_pool().await;

    // Execute the same query multiple times (should use prepared statements)
    for i in 0..10 {
        let result: (i32,) = sqlx::query_as("SELECT $1::int * 2")
            .bind(i)
            .fetch_one(pool.postgres())
            .await
            .unwrap();

        assert_eq!(result.0, i * 2);
    }
}

#[tokio::test]
async fn test_pool_isolation() {
    // Create two separate pools
    let (pool1, _guard1) = setup_test_pool().await;
    let (pool2, _guard2) = setup_test_pool().await;

    // Verify they are independent
    let stats1 = pool1.stats();
    let stats2 = pool2.stats();

    // Both pools should be functional
    assert!(stats1.postgres_size > 0);
    assert!(stats2.postgres_size > 0);

    // Execute queries on both pools
    let result1: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(pool1.postgres())
        .await
        .unwrap();
    let result2: (i32,) = sqlx::query_as("SELECT 2")
        .fetch_one(pool2.postgres())
        .await
        .unwrap();

    assert_eq!(result1.0, 1);
    assert_eq!(result2.0, 2);
}
