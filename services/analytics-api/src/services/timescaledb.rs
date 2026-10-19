use crate::models::*;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tracing::{debug, error, instrument};

/// Service for querying TimescaleDB analytics data
pub struct TimescaleDBService {
    pool: PgPool,
}

impl TimescaleDBService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get cost analytics for a given time range
    #[instrument(skip(self))]
    pub async fn get_cost_analytics(&self, query: &AnalyticsQuery) -> Result<CostAnalytics> {
        let (start_time, end_time) = self.get_time_range(query);
        let table = self.get_table_for_granularity(&query.granularity);

        debug!(
            "Querying cost analytics from {} to {} using table {}",
            start_time, end_time, table
        );

        // Build the WHERE clause
        let mut conditions = vec!["bucket >= $1".to_string(), "bucket <= $2".to_string()];
        let mut param_count = 3;

        if query.provider.is_some() {
            conditions.push(format!("provider = ${}", param_count));
            param_count += 1;
        }
        if query.model.is_some() {
            conditions.push(format!("model = ${}", param_count));
            param_count += 1;
        }
        if query.environment.is_some() {
            conditions.push(format!("environment = ${}", param_count));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Query for time series
        let time_series_query = format!(
            r#"
            SELECT
                bucket,
                COALESCE(SUM(total_cost_usd), 0) as total_cost_usd,
                COALESCE(SUM(prompt_cost_usd), 0) as prompt_cost_usd,
                COALESCE(SUM(completion_cost_usd), 0) as completion_cost_usd,
                SUM(request_count) as request_count
            FROM {}
            {}
            GROUP BY bucket
            ORDER BY bucket
            "#,
            table, where_clause
        );

        let mut query_builder = sqlx::query_as::<_, CostRow>(&time_series_query)
            .bind(start_time)
            .bind(end_time);

        if let Some(ref provider) = query.provider {
            query_builder = query_builder.bind(provider);
        }
        if let Some(ref model) = query.model {
            query_builder = query_builder.bind(model);
        }
        if let Some(ref environment) = query.environment {
            query_builder = query_builder.bind(environment);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        // Calculate totals
        let total_cost = rows
            .iter()
            .map(|r| r.total_cost_usd.unwrap_or(0.0))
            .sum::<f64>();
        let prompt_cost = rows
            .iter()
            .map(|r| r.prompt_cost_usd.unwrap_or(0.0))
            .sum::<f64>();
        let completion_cost = rows
            .iter()
            .map(|r| r.completion_cost_usd.unwrap_or(0.0))
            .sum::<f64>();
        let request_count = rows.iter().map(|r| r.request_count).sum::<i64>();

        let avg_cost_per_request = if request_count > 0 {
            total_cost / request_count as f64
        } else {
            0.0
        };

        // Convert to time series
        let time_series = rows
            .into_iter()
            .map(|row| CostDataPoint {
                timestamp: row.bucket,
                total_cost: row.total_cost_usd.unwrap_or(0.0),
                prompt_cost: row.prompt_cost_usd.unwrap_or(0.0),
                completion_cost: row.completion_cost_usd.unwrap_or(0.0),
                request_count: row.request_count,
            })
            .collect();

        Ok(CostAnalytics {
            total_cost,
            prompt_cost,
            completion_cost,
            request_count,
            avg_cost_per_request,
            time_series,
        })
    }

    /// Get cost breakdown by model, user, and provider
    #[instrument(skip(self))]
    pub async fn get_cost_breakdown(&self, query: &AnalyticsQuery) -> Result<CostBreakdown> {
        let (start_time, end_time) = self.get_time_range(query);
        let table = self.get_table_for_granularity(&query.granularity);

        // Build base WHERE clause
        let mut base_conditions = vec!["bucket >= $1".to_string(), "bucket <= $2".to_string()];
        let mut param_count = 3;

        if query.environment.is_some() {
            base_conditions.push(format!("environment = ${}", param_count));
            param_count += 1;
        }

        let base_where = if base_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", base_conditions.join(" AND "))
        };

        // Get total for percentage calculation
        let total_query = format!(
            "SELECT COALESCE(SUM(total_cost_usd), 0) as total FROM {} {}",
            table, base_where
        );

        let mut total_query_builder = sqlx::query_scalar::<_, f64>(&total_query)
            .bind(start_time)
            .bind(end_time);

        if query.environment.is_some() {
            total_query_builder = total_query_builder.bind(query.environment.as_ref().unwrap());
        }

        let total_cost: f64 = total_query_builder.fetch_one(&self.pool).await?;

        // Query breakdown by model
        let by_model_query = format!(
            r#"
            SELECT
                model as dimension,
                COALESCE(SUM(total_cost_usd), 0) as total_cost_usd,
                SUM(request_count) as request_count
            FROM {}
            {}
            GROUP BY model
            ORDER BY total_cost_usd DESC
            LIMIT 20
            "#,
            table, base_where
        );

        let mut by_model_builder = sqlx::query_as::<_, CostBreakdownRow>(&by_model_query)
            .bind(start_time)
            .bind(end_time);

        if query.environment.is_some() {
            by_model_builder = by_model_builder.bind(query.environment.as_ref().unwrap());
        }

        let by_model_rows = by_model_builder.fetch_all(&self.pool).await?;

        // Query breakdown by provider
        let by_provider_query = format!(
            r#"
            SELECT
                provider as dimension,
                COALESCE(SUM(total_cost_usd), 0) as total_cost_usd,
                SUM(request_count) as request_count
            FROM {}
            {}
            GROUP BY provider
            ORDER BY total_cost_usd DESC
            "#,
            table, base_where
        );

        let mut by_provider_builder = sqlx::query_as::<_, CostBreakdownRow>(&by_provider_query)
            .bind(start_time)
            .bind(end_time);

        if query.environment.is_some() {
            by_provider_builder = by_provider_builder.bind(query.environment.as_ref().unwrap());
        }

        let by_provider_rows = by_provider_builder.fetch_all(&self.pool).await?;

        // For by_user, we need to query raw traces if granularity allows
        let by_user = if query.granularity == "1min" || query.granularity == "raw" {
            let user_query = r#"
                SELECT
                    COALESCE(user_id, 'anonymous') as dimension,
                    COALESCE(SUM(total_cost_usd), 0) as total_cost_usd,
                    COUNT(*) as request_count
                FROM llm_traces
                WHERE ts >= $1 AND ts <= $2
                GROUP BY user_id
                ORDER BY total_cost_usd DESC
                LIMIT 20
            "#;

            sqlx::query_as::<_, CostBreakdownRow>(user_query)
                .bind(start_time)
                .bind(end_time)
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Get time series
        let time_series_query = format!(
            r#"
            SELECT
                bucket,
                'total' as provider,
                'all' as model,
                COALESCE(SUM(total_cost_usd), 0) as total_cost_usd,
                COALESCE(SUM(prompt_cost_usd), 0) as prompt_cost_usd,
                COALESCE(SUM(completion_cost_usd), 0) as completion_cost_usd,
                SUM(request_count) as request_count
            FROM {}
            {}
            GROUP BY bucket
            ORDER BY bucket
            "#,
            table, base_where
        );

        let mut time_series_builder = sqlx::query_as::<_, CostRow>(&time_series_query)
            .bind(start_time)
            .bind(end_time);

        if query.environment.is_some() {
            time_series_builder = time_series_builder.bind(query.environment.as_ref().unwrap());
        }

        let time_series_rows = time_series_builder.fetch_all(&self.pool).await?;

        Ok(CostBreakdown {
            by_model: self.convert_breakdown_rows(by_model_rows, total_cost),
            by_user: self.convert_breakdown_rows(by_user, total_cost),
            by_provider: self.convert_breakdown_rows(by_provider_rows, total_cost),
            by_time: time_series_rows
                .into_iter()
                .map(|row| CostDataPoint {
                    timestamp: row.bucket,
                    total_cost: row.total_cost_usd.unwrap_or(0.0),
                    prompt_cost: row.prompt_cost_usd.unwrap_or(0.0),
                    completion_cost: row.completion_cost_usd.unwrap_or(0.0),
                    request_count: row.request_count,
                })
                .collect(),
        })
    }

    /// Get performance metrics
    #[instrument(skip(self))]
    pub async fn get_performance_metrics(
        &self,
        query: &AnalyticsQuery,
    ) -> Result<PerformanceMetrics> {
        let (start_time, end_time) = self.get_time_range(query);
        let table = self.get_table_for_granularity(&query.granularity);

        // Build WHERE clause
        let mut conditions = vec!["bucket >= $1".to_string(), "bucket <= $2".to_string()];
        let mut param_count = 3;

        if query.provider.is_some() {
            conditions.push(format!("provider = ${}", param_count));
            param_count += 1;
        }
        if query.model.is_some() {
            conditions.push(format!("model = ${}", param_count));
            param_count += 1;
        }
        if query.environment.is_some() {
            conditions.push(format!("environment = ${}", param_count));
        }

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        // Query for time series
        let time_series_query = format!(
            r#"
            SELECT
                bucket,
                AVG(avg_duration_ms) as avg_duration_ms,
                MIN(min_duration_ms) as min_duration_ms,
                MAX(max_duration_ms) as max_duration_ms,
                SUM(request_count) as request_count,
                COALESCE(SUM(total_tokens), 0) as total_tokens
            FROM {}
            {}
            GROUP BY bucket
            ORDER BY bucket
            "#,
            table, where_clause
        );

        let mut query_builder = sqlx::query_as::<_, PerformanceRow>(&time_series_query)
            .bind(start_time)
            .bind(end_time);

        if let Some(ref provider) = query.provider {
            query_builder = query_builder.bind(provider);
        }
        if let Some(ref model) = query.model {
            query_builder = query_builder.bind(model);
        }
        if let Some(ref environment) = query.environment {
            query_builder = query_builder.bind(environment);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        // Calculate aggregates
        let request_count: i64 = rows.iter().map(|r| r.request_count).sum();
        let total_tokens: i64 = rows.iter().map(|r| r.total_tokens.unwrap_or(0)).sum();

        let avg_latency_ms = if !rows.is_empty() {
            rows.iter()
                .map(|r| r.avg_duration_ms.unwrap_or(0.0))
                .sum::<f64>()
                / rows.len() as f64
        } else {
            0.0
        };

        let min_latency_ms = rows
            .iter()
            .filter_map(|r| r.min_duration_ms)
            .min()
            .unwrap_or(0);

        let max_latency_ms = rows
            .iter()
            .filter_map(|r| r.max_duration_ms)
            .max()
            .unwrap_or(0);

        // Calculate throughput
        let duration_seconds = (end_time - start_time).num_seconds() as f64;
        let throughput_rps = if duration_seconds > 0.0 {
            request_count as f64 / duration_seconds
        } else {
            0.0
        };

        let tokens_per_second = if duration_seconds > 0.0 {
            total_tokens as f64 / duration_seconds
        } else {
            0.0
        };

        // Calculate percentiles from raw data if needed
        let percentiles = if query.granularity == "1min" || query.granularity == "raw" {
            self.calculate_percentiles(query).await?
        } else {
            PercentileMetrics {
                p50: None,
                p95: None,
                p99: None,
            }
        };

        // Convert to time series
        let time_series = rows
            .into_iter()
            .map(|row| PerformanceDataPoint {
                timestamp: row.bucket,
                avg_latency_ms: row.avg_duration_ms.unwrap_or(0.0),
                min_latency_ms: row.min_duration_ms.unwrap_or(0),
                max_latency_ms: row.max_duration_ms.unwrap_or(0),
                request_count: row.request_count,
                total_tokens: row.total_tokens.unwrap_or(0),
            })
            .collect();

        Ok(PerformanceMetrics {
            request_count,
            avg_latency_ms,
            min_latency_ms,
            max_latency_ms,
            p50_latency_ms: percentiles.p50,
            p95_latency_ms: percentiles.p95,
            p99_latency_ms: percentiles.p99,
            throughput_rps,
            total_tokens,
            tokens_per_second,
            time_series,
        })
    }

    /// Calculate percentiles from raw trace data
    #[instrument(skip(self))]
    async fn calculate_percentiles(&self, query: &AnalyticsQuery) -> Result<PercentileMetrics> {
        let (start_time, end_time) = self.get_time_range(query);

        let mut conditions = vec!["ts >= $1".to_string(), "ts <= $2".to_string()];
        let mut param_count = 3;

        if query.provider.is_some() {
            conditions.push(format!("provider = ${}", param_count));
            param_count += 1;
        }
        if query.model.is_some() {
            conditions.push(format!("model = ${}", param_count));
            param_count += 1;
        }
        if query.environment.is_some() {
            conditions.push(format!("environment = ${}", param_count));
        }

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let percentile_query = format!(
            r#"
            SELECT
                PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms) as p50,
                PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95,
                PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) as p99
            FROM llm_traces
            {}
            "#,
            where_clause
        );

        let mut query_builder = sqlx::query_as::<_, PercentileMetrics>(&percentile_query)
            .bind(start_time)
            .bind(end_time);

        if let Some(ref provider) = query.provider {
            query_builder = query_builder.bind(provider);
        }
        if let Some(ref model) = query.model {
            query_builder = query_builder.bind(model);
        }
        if let Some(ref environment) = query.environment {
            query_builder = query_builder.bind(environment);
        }

        let percentiles = query_builder.fetch_one(&self.pool).await?;

        Ok(percentiles)
    }

    /// Get quality metrics
    #[instrument(skip(self))]
    pub async fn get_quality_metrics(&self, query: &AnalyticsQuery) -> Result<QualityMetrics> {
        let (start_time, end_time) = self.get_time_range(query);
        let table = self.get_table_for_granularity(&query.granularity);

        // Build WHERE clause
        let mut conditions = vec!["bucket >= $1".to_string(), "bucket <= $2".to_string()];
        let mut param_count = 3;

        if query.provider.is_some() {
            conditions.push(format!("provider = ${}", param_count));
            param_count += 1;
        }
        if query.model.is_some() {
            conditions.push(format!("model = ${}", param_count));
            param_count += 1;
        }
        if query.environment.is_some() {
            conditions.push(format!("environment = ${}", param_count));
        }

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        // Query for time series
        let time_series_query = format!(
            r#"
            SELECT
                bucket,
                SUM(request_count) as request_count,
                COALESCE(SUM(success_count), 0) as success_count,
                COALESCE(SUM(error_count), 0) as error_count
            FROM {}
            {}
            GROUP BY bucket
            ORDER BY bucket
            "#,
            table, where_clause
        );

        let mut query_builder = sqlx::query_as::<_, QualityRow>(&time_series_query)
            .bind(start_time)
            .bind(end_time);

        if let Some(ref provider) = query.provider {
            query_builder = query_builder.bind(provider);
        }
        if let Some(ref model) = query.model {
            query_builder = query_builder.bind(model);
        }
        if let Some(ref environment) = query.environment {
            query_builder = query_builder.bind(environment);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        // Calculate totals
        let total_requests: i64 = rows.iter().map(|r| r.request_count).sum();
        let successful_requests: i64 = rows.iter().map(|r| r.success_count.unwrap_or(0)).sum();
        let failed_requests: i64 = rows.iter().map(|r| r.error_count.unwrap_or(0)).sum();

        let success_rate = if total_requests > 0 {
            successful_requests as f64 / total_requests as f64
        } else {
            0.0
        };

        let error_rate = if total_requests > 0 {
            failed_requests as f64 / total_requests as f64
        } else {
            0.0
        };

        // Get error breakdown
        let error_breakdown = self.get_error_breakdown(query).await?;

        // Convert to time series
        let time_series = rows
            .into_iter()
            .map(|row| {
                let req_count = row.request_count;
                let success = row.success_count.unwrap_or(0);
                let errors = row.error_count.unwrap_or(0);

                QualityDataPoint {
                    timestamp: row.bucket,
                    success_rate: if req_count > 0 {
                        success as f64 / req_count as f64
                    } else {
                        0.0
                    },
                    error_rate: if req_count > 0 {
                        errors as f64 / req_count as f64
                    } else {
                        0.0
                    },
                    request_count: req_count,
                }
            })
            .collect();

        Ok(QualityMetrics {
            total_requests,
            successful_requests,
            failed_requests,
            success_rate,
            error_rate,
            avg_feedback_score: None,
            resolution_rate: None,
            error_breakdown,
            time_series,
        })
    }

    /// Get error breakdown
    async fn get_error_breakdown(&self, query: &AnalyticsQuery) -> Result<Vec<ErrorBreakdownItem>> {
        let (start_time, end_time) = self.get_time_range(query);

        let mut conditions = vec!["bucket >= $1".to_string(), "bucket <= $2".to_string()];
        let mut param_count = 3;

        if query.provider.is_some() {
            conditions.push(format!("provider = ${}", param_count));
            param_count += 1;
        }
        if query.model.is_some() {
            conditions.push(format!("model = ${}", param_count));
            param_count += 1;
        }
        if query.environment.is_some() {
            conditions.push(format!("environment = ${}", param_count));
        }

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let error_query = format!(
            r#"
            SELECT
                status_code,
                SUM(error_count) as error_count,
                MIN(sample_error_message) as sample_error_message
            FROM llm_error_summary
            {}
            GROUP BY status_code
            ORDER BY error_count DESC
            LIMIT 10
            "#,
            where_clause
        );

        let mut query_builder = sqlx::query_as::<_, ErrorBreakdownRow>(&error_query)
            .bind(start_time)
            .bind(end_time);

        if let Some(ref provider) = query.provider {
            query_builder = query_builder.bind(provider);
        }
        if let Some(ref model) = query.model {
            query_builder = query_builder.bind(model);
        }
        if let Some(ref environment) = query.environment {
            query_builder = query_builder.bind(environment);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let total_errors: i64 = rows.iter().map(|r| r.error_count).sum();

        Ok(rows
            .into_iter()
            .map(|row| ErrorBreakdownItem {
                error_type: row.status_code,
                count: row.error_count,
                percentage: if total_errors > 0 {
                    (row.error_count as f64 / total_errors as f64) * 100.0
                } else {
                    0.0
                },
                sample_message: row.sample_error_message,
            })
            .collect())
    }

    /// Compare multiple models
    #[instrument(skip(self))]
    pub async fn compare_models(&self, query: &ModelComparisonQuery) -> Result<ModelComparison> {
        let start_time = query
            .start_time
            .unwrap_or_else(|| Utc::now() - Duration::days(7));
        let end_time = query.end_time.unwrap_or_else(Utc::now);

        let mut results = Vec::new();

        for model in &query.models {
            let model_query = r#"
                SELECT
                    provider,
                    model,
                    AVG(duration_ms) as avg_duration_ms,
                    COALESCE(SUM(total_cost_usd), 0) as total_cost_usd,
                    SUM(total_tokens) as total_tokens,
                    COUNT(*) as request_count,
                    SUM(CASE WHEN status_code = 'OK' THEN 1 ELSE 0 END) as success_count
                FROM llm_traces
                WHERE ts >= $1 AND ts <= $2 AND model = $3
                GROUP BY provider, model
            "#;

            let row = sqlx::query_as::<_, ModelMetricsRow>(model_query)
                .bind(start_time)
                .bind(end_time)
                .bind(model)
                .fetch_optional(&self.pool)
                .await?;

            if let Some(row) = row {
                let success_rate = if row.request_count > 0 {
                    row.success_count.unwrap_or(0) as f64 / row.request_count as f64
                } else {
                    0.0
                };

                let avg_cost_usd = if row.request_count > 0 {
                    row.total_cost_usd.unwrap_or(0.0) / row.request_count as f64
                } else {
                    0.0
                };

                let duration_seconds = (end_time - start_time).num_seconds() as f64;
                let throughput_rps = if duration_seconds > 0.0 {
                    row.request_count as f64 / duration_seconds
                } else {
                    0.0
                };

                // Calculate percentiles
                let percentile_query = r#"
                    SELECT
                        PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95
                    FROM llm_traces
                    WHERE ts >= $1 AND ts <= $2 AND model = $3
                "#;

                let p95: Option<f64> = sqlx::query_scalar(percentile_query)
                    .bind(start_time)
                    .bind(end_time)
                    .bind(model)
                    .fetch_optional(&self.pool)
                    .await?
                    .flatten();

                results.push(ModelComparisonResult {
                    model: row.model.clone(),
                    provider: row.provider.clone(),
                    metrics: ModelMetrics {
                        avg_latency_ms: row.avg_duration_ms.unwrap_or(0.0),
                        p95_latency_ms: p95,
                        avg_cost_usd,
                        total_cost_usd: row.total_cost_usd.unwrap_or(0.0),
                        success_rate,
                        request_count: row.request_count,
                        total_tokens: row.total_tokens.unwrap_or(0),
                        throughput_rps,
                    },
                });
            }
        }

        // Generate summary
        let summary = self.generate_comparison_summary(&results);

        Ok(ModelComparison {
            models: results,
            summary,
        })
    }

    /// Generate comparison summary with recommendations
    fn generate_comparison_summary(
        &self,
        results: &[ModelComparisonResult],
    ) -> ModelComparisonSummary {
        if results.is_empty() {
            return ModelComparisonSummary {
                fastest_model: "N/A".to_string(),
                cheapest_model: "N/A".to_string(),
                most_reliable_model: "N/A".to_string(),
                recommendations: vec![],
            };
        }

        let fastest = results
            .iter()
            .min_by(|a, b| {
                a.metrics
                    .avg_latency_ms
                    .partial_cmp(&b.metrics.avg_latency_ms)
                    .unwrap()
            })
            .map(|r| r.model.clone())
            .unwrap_or_default();

        let cheapest = results
            .iter()
            .min_by(|a, b| {
                a.metrics
                    .avg_cost_usd
                    .partial_cmp(&b.metrics.avg_cost_usd)
                    .unwrap()
            })
            .map(|r| r.model.clone())
            .unwrap_or_default();

        let most_reliable = results
            .iter()
            .max_by(|a, b| {
                a.metrics
                    .success_rate
                    .partial_cmp(&b.metrics.success_rate)
                    .unwrap()
            })
            .map(|r| r.model.clone())
            .unwrap_or_default();

        let mut recommendations = Vec::new();

        // Generate recommendations based on metrics
        if fastest != cheapest {
            recommendations.push(format!(
                "Consider using {} for latency-sensitive applications and {} for cost optimization",
                fastest, cheapest
            ));
        }

        if let Some(best) = results.iter().find(|r| {
            r.metrics.success_rate > 0.99
                && r.metrics.avg_latency_ms < 2000.0
                && r.metrics.avg_cost_usd < 0.01
        }) {
            recommendations.push(format!(
                "{} offers the best balance of speed, cost, and reliability",
                best.model
            ));
        }

        ModelComparisonSummary {
            fastest_model: fastest,
            cheapest_model: cheapest,
            most_reliable_model: most_reliable,
            recommendations,
        }
    }

    /// Get optimization recommendations
    #[instrument(skip(self))]
    pub async fn get_optimization_recommendations(
        &self,
        query: &AnalyticsQuery,
    ) -> Result<OptimizationRecommendations> {
        let cost_analytics = self.get_cost_analytics(query).await?;
        let performance_metrics = self.get_performance_metrics(query).await?;
        let quality_metrics = self.get_quality_metrics(query).await?;

        let mut cost_optimizations = Vec::new();
        let mut performance_optimizations = Vec::new();
        let mut quality_optimizations = Vec::new();

        // Cost recommendations
        if cost_analytics.avg_cost_per_request > 0.01 {
            cost_optimizations.push(Recommendation {
                title: "High per-request cost detected".to_string(),
                description: format!(
                    "Average cost per request is ${:.4}. Consider using smaller models for simple tasks.",
                    cost_analytics.avg_cost_per_request
                ),
                impact: ImpactLevel::High,
                potential_savings: Some(cost_analytics.total_cost * 0.3),
                effort: EffortLevel::Medium,
                priority: 1,
            });
        }

        // Performance recommendations
        if performance_metrics.avg_latency_ms > 2000.0 {
            performance_optimizations.push(Recommendation {
                title: "High latency detected".to_string(),
                description: format!(
                    "Average latency is {:.0}ms. Consider implementing caching or using faster models.",
                    performance_metrics.avg_latency_ms
                ),
                impact: ImpactLevel::High,
                potential_savings: None,
                effort: EffortLevel::Medium,
                priority: 1,
            });
        }

        // Quality recommendations
        if quality_metrics.error_rate > 0.05 {
            quality_optimizations.push(Recommendation {
                title: "High error rate detected".to_string(),
                description: format!(
                    "Error rate is {:.1}%. Investigate error patterns and implement retry logic.",
                    quality_metrics.error_rate * 100.0
                ),
                impact: ImpactLevel::High,
                potential_savings: None,
                effort: EffortLevel::High,
                priority: 1,
            });
        }

        // Calculate overall score
        let cost_score = if cost_analytics.avg_cost_per_request < 0.005 {
            1.0
        } else {
            0.5
        };
        let perf_score = if performance_metrics.avg_latency_ms < 1000.0 {
            1.0
        } else {
            0.5
        };
        let quality_score = if quality_metrics.success_rate > 0.95 {
            1.0
        } else {
            0.5
        };
        let overall_score = (cost_score + perf_score + quality_score) / 3.0;

        Ok(OptimizationRecommendations {
            cost_optimizations,
            performance_optimizations,
            quality_optimizations,
            overall_score,
        })
    }

    /// Helper: Get time range with defaults
    fn get_time_range(&self, query: &AnalyticsQuery) -> (DateTime<Utc>, DateTime<Utc>) {
        let end_time = query.end_time.unwrap_or_else(Utc::now);
        let start_time = query
            .start_time
            .unwrap_or_else(|| end_time - Duration::days(7));
        (start_time, end_time)
    }

    /// Helper: Get table name based on granularity
    fn get_table_for_granularity(&self, granularity: &str) -> &'static str {
        match granularity {
            "1min" => "llm_metrics_1min",
            "1hour" => "llm_metrics_1hour",
            "1day" => "llm_metrics_1day",
            _ => "llm_metrics_1hour",
        }
    }

    /// Helper: Convert breakdown rows to items
    fn convert_breakdown_rows(
        &self,
        rows: Vec<CostBreakdownRow>,
        total: f64,
    ) -> Vec<CostBreakdownItem> {
        rows.into_iter()
            .map(|row| CostBreakdownItem {
                dimension: row.dimension,
                total_cost: row.total_cost_usd.unwrap_or(0.0),
                request_count: row.request_count,
                percentage: if total > 0.0 {
                    (row.total_cost_usd.unwrap_or(0.0) / total) * 100.0
                } else {
                    0.0
                },
            })
            .collect()
    }
}
