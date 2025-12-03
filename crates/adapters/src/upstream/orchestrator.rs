// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM-Orchestrator adapter for Observatory.
//!
//! This module provides runtime integration for consuming telemetry from
//! the LLM-Orchestrator system, which manages workflow execution and
//! pipeline traces for complex LLM operations.
//!
//! # Features
//!
//! - Workflow telemetry consumption
//! - Pipeline execution trace processing
//! - Step-by-step execution tracking
//! - Orchestration metrics aggregation
//!
//! # Architecture
//!
//! This is a runtime-only adapter that processes workflow and pipeline data
//! without requiring compile-time dependencies on the upstream crate.
//! Data is consumed via standardized formats (JSON, OpenTelemetry).
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::orchestrator::OrchestratorAdapter;
//!
//! let adapter = OrchestratorAdapter::new("orchestrator-1");
//!
//! // Process workflow telemetry
//! let workflow = adapter.parse_workflow_telemetry(&json_data)?;
//!
//! // Extract pipeline traces
//! let traces = adapter.extract_pipeline_traces(&workflow)?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during orchestrator operations.
#[derive(Debug, Error)]
pub enum OrchestratorAdapterError {
    /// Invalid workflow data
    #[error("Invalid workflow data: {0}")]
    InvalidWorkflow(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Pipeline execution error
    #[error("Pipeline execution error: {0}")]
    PipelineError(String),

    /// Step execution error
    #[error("Step execution error: {0}")]
    StepError(String),
}

/// Result type for orchestrator operations.
pub type Result<T> = std::result::Result<T, OrchestratorAdapterError>;

/// Orchestrator identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrchestratorId(String);

impl OrchestratorId {
    /// Create a new orchestrator ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OrchestratorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Workflow identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(String);

impl WorkflowId {
    /// Create a new workflow ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Pipeline identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PipelineId(String);

impl PipelineId {
    /// Create a new pipeline ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Workflow telemetry from orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTelemetry {
    /// Workflow ID
    pub workflow_id: WorkflowId,
    /// Workflow name
    pub name: String,
    /// Orchestrator that executed this workflow
    pub orchestrator_id: OrchestratorId,
    /// Trace ID for distributed tracing
    pub trace_id: Option<String>,
    /// Workflow version
    pub version: Option<String>,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Total duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Workflow status
    pub status: WorkflowStatus,
    /// Pipeline executions within this workflow
    pub pipelines: Vec<PipelineExecution>,
    /// Total token usage across all pipelines
    pub total_token_usage: Option<WorkflowTokenUsage>,
    /// Total cost across all pipelines
    pub total_cost_usd: Option<f64>,
    /// Input parameters
    pub input_params: HashMap<String, serde_json::Value>,
    /// Output results
    pub output_results: HashMap<String, serde_json::Value>,
    /// Workflow metadata
    pub metadata: HashMap<String, String>,
}

/// Workflow execution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Workflow is pending execution
    Pending,
    /// Workflow is currently running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
    /// Workflow timed out
    Timeout,
    /// Workflow is paused
    Paused,
}

/// Token usage aggregated at workflow level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowTokenUsage {
    /// Total prompt tokens
    pub total_prompt_tokens: u64,
    /// Total completion tokens
    pub total_completion_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
    /// Tokens by model
    pub by_model: HashMap<String, u64>,
    /// Tokens by pipeline
    pub by_pipeline: HashMap<String, u64>,
}

/// Pipeline execution within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineExecution {
    /// Pipeline ID
    pub pipeline_id: PipelineId,
    /// Pipeline name
    pub name: String,
    /// Parent workflow ID
    pub workflow_id: WorkflowId,
    /// Span ID for this pipeline
    pub span_id: String,
    /// Parent span ID
    pub parent_span_id: Option<String>,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Pipeline status
    pub status: PipelineStatus,
    /// Steps in this pipeline
    pub steps: Vec<PipelineStep>,
    /// Total tokens used
    pub token_usage: Option<PipelineTokenUsage>,
    /// Cost for this pipeline
    pub cost_usd: Option<f64>,
    /// Error information
    pub error: Option<PipelineError>,
}

/// Pipeline execution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    /// Pipeline is pending
    Pending,
    /// Pipeline is running
    Running,
    /// Pipeline completed successfully
    Completed,
    /// Pipeline failed
    Failed,
    /// Pipeline was skipped (conditional)
    Skipped,
    /// Pipeline was retried
    Retried,
}

/// Token usage for a pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineTokenUsage {
    /// Prompt tokens
    pub prompt_tokens: u64,
    /// Completion tokens
    pub completion_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
}

/// Pipeline error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Step where error occurred
    pub step_id: Option<String>,
    /// Is error retryable
    pub retryable: bool,
}

/// Individual step within a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    /// Step ID
    pub step_id: String,
    /// Step name
    pub name: String,
    /// Step type
    pub step_type: StepType,
    /// Span ID
    pub span_id: String,
    /// Parent span ID
    pub parent_span_id: Option<String>,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Step status
    pub status: StepStatus,
    /// Model used (for LLM steps)
    pub model: Option<String>,
    /// Provider (for LLM steps)
    pub provider: Option<String>,
    /// Token usage (for LLM steps)
    pub token_usage: Option<StepTokenUsage>,
    /// Input to step
    pub input: Option<serde_json::Value>,
    /// Output from step
    pub output: Option<serde_json::Value>,
    /// Step attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Type of pipeline step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// LLM completion call
    LlmCompletion,
    /// LLM chat call
    LlmChat,
    /// LLM embedding call
    LlmEmbedding,
    /// Data transformation
    Transform,
    /// External API call
    ApiCall,
    /// Database operation
    Database,
    /// Cache operation
    Cache,
    /// Conditional branching
    Condition,
    /// Parallel execution
    Parallel,
    /// Loop/iteration
    Loop,
    /// Custom step type
    Custom(String),
}

/// Step execution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step is pending
    Pending,
    /// Step is running
    Running,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Step was skipped
    Skipped,
    /// Step is waiting on dependencies
    Waiting,
}

/// Token usage for a step.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepTokenUsage {
    /// Prompt tokens
    pub prompt_tokens: u32,
    /// Completion tokens
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

/// Orchestrator statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrchestratorStats {
    /// Total workflows processed
    pub total_workflows: u64,
    /// Completed workflows
    pub completed_workflows: u64,
    /// Failed workflows
    pub failed_workflows: u64,
    /// Total pipelines executed
    pub total_pipelines: u64,
    /// Total steps executed
    pub total_steps: u64,
    /// Total LLM calls
    pub total_llm_calls: u64,
    /// Average workflow duration (ms)
    pub avg_workflow_duration_ms: f64,
    /// Average pipeline duration (ms)
    pub avg_pipeline_duration_ms: f64,
    /// Total tokens consumed
    pub total_tokens: u64,
    /// Total cost (USD)
    pub total_cost_usd: f64,
}

/// Adapter for consuming LLM-Orchestrator telemetry.
///
/// Provides runtime integration for Observatory to ingest workflow telemetry
/// and pipeline traces from orchestrators without compile-time dependencies.
pub struct OrchestratorAdapter {
    /// Orchestrator identifier
    orchestrator_id: OrchestratorId,
    /// Collected workflow telemetry
    workflows: Vec<WorkflowTelemetry>,
    /// Statistics
    stats: OrchestratorStats,
}

impl OrchestratorAdapter {
    /// Create a new OrchestratorAdapter.
    pub fn new(orchestrator_id: impl Into<String>) -> Self {
        Self {
            orchestrator_id: OrchestratorId::new(orchestrator_id),
            workflows: Vec::new(),
            stats: OrchestratorStats::default(),
        }
    }

    /// Get the orchestrator ID.
    pub fn orchestrator_id(&self) -> &OrchestratorId {
        &self.orchestrator_id
    }

    /// Parse workflow telemetry from JSON.
    pub fn parse_workflow_telemetry(
        &mut self,
        json_data: &serde_json::Value,
    ) -> Result<WorkflowTelemetry> {
        let workflow_id = json_data
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OrchestratorAdapterError::MissingField("workflow_id".to_string()))?;

        let name = json_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed-workflow")
            .to_string();

        let status = json_data
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "pending" => WorkflowStatus::Pending,
                "running" => WorkflowStatus::Running,
                "completed" => WorkflowStatus::Completed,
                "failed" => WorkflowStatus::Failed,
                "cancelled" => WorkflowStatus::Cancelled,
                "timeout" => WorkflowStatus::Timeout,
                "paused" => WorkflowStatus::Paused,
                _ => WorkflowStatus::Pending,
            })
            .unwrap_or(WorkflowStatus::Pending);

        let pipelines = self.parse_pipelines(json_data, &WorkflowId::new(workflow_id))?;

        let total_token_usage = self.aggregate_token_usage(&pipelines);
        let total_cost_usd = self.aggregate_cost(&pipelines);

        let workflow = WorkflowTelemetry {
            workflow_id: WorkflowId::new(workflow_id),
            name,
            orchestrator_id: self.orchestrator_id.clone(),
            trace_id: json_data
                .get("trace_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            version: json_data
                .get("version")
                .and_then(|v| v.as_str())
                .map(String::from),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: json_data.get("duration_ms").and_then(|v| v.as_u64()),
            status: status.clone(),
            pipelines,
            total_token_usage: Some(total_token_usage),
            total_cost_usd: Some(total_cost_usd),
            input_params: json_data
                .get("input_params")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            output_results: json_data
                .get("output_results")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            metadata: json_data
                .get("metadata")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
        };

        // Update statistics
        self.stats.total_workflows += 1;
        match status {
            WorkflowStatus::Completed => self.stats.completed_workflows += 1,
            WorkflowStatus::Failed => self.stats.failed_workflows += 1,
            _ => {}
        }

        if let Some(duration) = workflow.duration_ms {
            let n = self.stats.total_workflows as f64;
            self.stats.avg_workflow_duration_ms =
                (self.stats.avg_workflow_duration_ms * (n - 1.0) + duration as f64) / n;
        }

        self.workflows.push(workflow.clone());

        Ok(workflow)
    }

    /// Parse pipelines from workflow JSON.
    fn parse_pipelines(
        &mut self,
        json_data: &serde_json::Value,
        workflow_id: &WorkflowId,
    ) -> Result<Vec<PipelineExecution>> {
        let pipelines_array = match json_data.get("pipelines") {
            Some(arr) if arr.is_array() => arr.as_array().unwrap(),
            _ => return Ok(Vec::new()),
        };

        let mut pipelines = Vec::new();

        for pipeline_json in pipelines_array {
            let pipeline_id = pipeline_json
                .get("pipeline_id")
                .and_then(|v| v.as_str())
                .unwrap_or(&Uuid::new_v4().to_string())
                .to_string();

            let name = pipeline_json
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed-pipeline")
                .to_string();

            let status = pipeline_json
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "pending" => PipelineStatus::Pending,
                    "running" => PipelineStatus::Running,
                    "completed" => PipelineStatus::Completed,
                    "failed" => PipelineStatus::Failed,
                    "skipped" => PipelineStatus::Skipped,
                    "retried" => PipelineStatus::Retried,
                    _ => PipelineStatus::Pending,
                })
                .unwrap_or(PipelineStatus::Pending);

            let steps = self.parse_steps(pipeline_json)?;

            let token_usage = self.aggregate_step_tokens(&steps);

            let pipeline = PipelineExecution {
                pipeline_id: PipelineId::new(&pipeline_id),
                name,
                workflow_id: workflow_id.clone(),
                span_id: pipeline_json
                    .get("span_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&Uuid::new_v4().to_string())
                    .to_string(),
                parent_span_id: pipeline_json
                    .get("parent_span_id")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                start_time: Utc::now(),
                end_time: None,
                duration_ms: pipeline_json.get("duration_ms").and_then(|v| v.as_u64()),
                status,
                steps,
                token_usage: Some(token_usage),
                cost_usd: pipeline_json.get("cost_usd").and_then(|v| v.as_f64()),
                error: None,
            };

            self.stats.total_pipelines += 1;
            pipelines.push(pipeline);
        }

        Ok(pipelines)
    }

    /// Parse steps from pipeline JSON.
    fn parse_steps(&mut self, pipeline_json: &serde_json::Value) -> Result<Vec<PipelineStep>> {
        let steps_array = match pipeline_json.get("steps") {
            Some(arr) if arr.is_array() => arr.as_array().unwrap(),
            _ => return Ok(Vec::new()),
        };

        let mut steps = Vec::new();

        for step_json in steps_array {
            let step_type = step_json
                .get("step_type")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "llm_completion" => StepType::LlmCompletion,
                    "llm_chat" => StepType::LlmChat,
                    "llm_embedding" => StepType::LlmEmbedding,
                    "transform" => StepType::Transform,
                    "api_call" => StepType::ApiCall,
                    "database" => StepType::Database,
                    "cache" => StepType::Cache,
                    "condition" => StepType::Condition,
                    "parallel" => StepType::Parallel,
                    "loop" => StepType::Loop,
                    other => StepType::Custom(other.to_string()),
                })
                .unwrap_or(StepType::Custom("unknown".to_string()));

            let status = step_json
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "pending" => StepStatus::Pending,
                    "running" => StepStatus::Running,
                    "completed" => StepStatus::Completed,
                    "failed" => StepStatus::Failed,
                    "skipped" => StepStatus::Skipped,
                    "waiting" => StepStatus::Waiting,
                    _ => StepStatus::Pending,
                })
                .unwrap_or(StepStatus::Pending);

            let token_usage = step_json.get("token_usage").and_then(|v| {
                Some(StepTokenUsage {
                    prompt_tokens: v.get("prompt_tokens")?.as_u64()? as u32,
                    completion_tokens: v.get("completion_tokens")?.as_u64()? as u32,
                    total_tokens: v.get("total_tokens")?.as_u64()? as u32,
                })
            });

            let step = PipelineStep {
                step_id: step_json
                    .get("step_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&Uuid::new_v4().to_string())
                    .to_string(),
                name: step_json
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unnamed-step")
                    .to_string(),
                step_type: step_type.clone(),
                span_id: step_json
                    .get("span_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&Uuid::new_v4().to_string())
                    .to_string(),
                parent_span_id: step_json
                    .get("parent_span_id")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                start_time: Utc::now(),
                end_time: None,
                duration_ms: step_json.get("duration_ms").and_then(|v| v.as_u64()),
                status,
                model: step_json
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                provider: step_json
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                token_usage,
                input: step_json.get("input").cloned(),
                output: step_json.get("output").cloned(),
                attributes: HashMap::new(),
            };

            self.stats.total_steps += 1;

            // Track LLM calls
            if matches!(
                step_type,
                StepType::LlmCompletion | StepType::LlmChat | StepType::LlmEmbedding
            ) {
                self.stats.total_llm_calls += 1;
            }

            steps.push(step);
        }

        Ok(steps)
    }

    /// Aggregate token usage from pipelines.
    fn aggregate_token_usage(&self, pipelines: &[PipelineExecution]) -> WorkflowTokenUsage {
        let mut usage = WorkflowTokenUsage::default();

        for pipeline in pipelines {
            if let Some(pu) = &pipeline.token_usage {
                usage.total_prompt_tokens += pu.prompt_tokens;
                usage.total_completion_tokens += pu.completion_tokens;
                usage.total_tokens += pu.total_tokens;
                *usage.by_pipeline.entry(pipeline.name.clone()).or_insert(0) += pu.total_tokens;
            }

            for step in &pipeline.steps {
                if let (Some(model), Some(tu)) = (&step.model, &step.token_usage) {
                    *usage.by_model.entry(model.clone()).or_insert(0) += tu.total_tokens as u64;
                }
            }
        }

        self.stats.total_tokens.saturating_add(usage.total_tokens);

        usage
    }

    /// Aggregate cost from pipelines.
    fn aggregate_cost(&self, pipelines: &[PipelineExecution]) -> f64 {
        let cost: f64 = pipelines.iter().filter_map(|p| p.cost_usd).sum();
        cost
    }

    /// Aggregate token usage from steps.
    fn aggregate_step_tokens(&self, steps: &[PipelineStep]) -> PipelineTokenUsage {
        let mut usage = PipelineTokenUsage::default();

        for step in steps {
            if let Some(tu) = &step.token_usage {
                usage.prompt_tokens += tu.prompt_tokens as u64;
                usage.completion_tokens += tu.completion_tokens as u64;
                usage.total_tokens += tu.total_tokens as u64;
            }
        }

        usage
    }

    /// Get all workflows.
    pub fn workflows(&self) -> &[WorkflowTelemetry] {
        &self.workflows
    }

    /// Get statistics.
    pub fn stats(&self) -> &OrchestratorStats {
        &self.stats
    }

    /// Clear all collected data.
    pub fn clear(&mut self) {
        self.workflows.clear();
        self.stats = OrchestratorStats::default();
    }

    /// Extract all pipeline executions across workflows.
    pub fn all_pipelines(&self) -> Vec<&PipelineExecution> {
        self.workflows
            .iter()
            .flat_map(|w| w.pipelines.iter())
            .collect()
    }

    /// Extract all steps across all pipelines.
    pub fn all_steps(&self) -> Vec<&PipelineStep> {
        self.workflows
            .iter()
            .flat_map(|w| w.pipelines.iter())
            .flat_map(|p| p.steps.iter())
            .collect()
    }

    /// Get all LLM steps.
    pub fn llm_steps(&self) -> Vec<&PipelineStep> {
        self.all_steps()
            .into_iter()
            .filter(|s| {
                matches!(
                    s.step_type,
                    StepType::LlmCompletion | StepType::LlmChat | StepType::LlmEmbedding
                )
            })
            .collect()
    }

    /// Check if workflow should be sampled (for tail-based sampling).
    pub fn should_sample_workflow(&self, workflow: &WorkflowTelemetry) -> bool {
        // Always sample failed workflows
        if workflow.status == WorkflowStatus::Failed {
            return true;
        }

        // Always sample timed out workflows
        if workflow.status == WorkflowStatus::Timeout {
            return true;
        }

        // Sample slow workflows (> 30 seconds)
        if let Some(duration) = workflow.duration_ms {
            if duration > 30000 {
                return true;
            }
        }

        // Sample high-cost workflows (> $1)
        if let Some(cost) = workflow.total_cost_usd {
            if cost > 1.0 {
                return true;
            }
        }

        // Sample high-token workflows (> 50K tokens)
        if let Some(usage) = &workflow.total_token_usage {
            if usage.total_tokens > 50000 {
                return true;
            }
        }

        // Sample workflows with failed pipelines
        if workflow
            .pipelines
            .iter()
            .any(|p| p.status == PipelineStatus::Failed)
        {
            return true;
        }

        false
    }

    /// Convert workflow to Observatory span format.
    pub fn workflow_to_span_json(&self, workflow: &WorkflowTelemetry) -> serde_json::Value {
        let child_spans: Vec<serde_json::Value> = workflow
            .pipelines
            .iter()
            .map(|p| self.pipeline_to_span_json(p))
            .collect();

        serde_json::json!({
            "trace_id": workflow.trace_id,
            "span_id": workflow.workflow_id.as_str(),
            "name": format!("workflow.{}", workflow.name),
            "start_time": workflow.start_time.to_rfc3339(),
            "end_time": workflow.end_time.map(|t| t.to_rfc3339()),
            "duration_ms": workflow.duration_ms,
            "status": match workflow.status {
                WorkflowStatus::Completed => "ok",
                _ => "error"
            },
            "token_usage": workflow.total_token_usage.as_ref().map(|u| serde_json::json!({
                "total_tokens": u.total_tokens
            })),
            "cost_usd": workflow.total_cost_usd,
            "attributes": {
                "orchestrator.id": self.orchestrator_id.as_str(),
                "workflow.version": workflow.version,
                "workflow.pipeline_count": workflow.pipelines.len()
            },
            "children": child_spans
        })
    }

    /// Convert pipeline to span JSON.
    pub fn pipeline_to_span_json(&self, pipeline: &PipelineExecution) -> serde_json::Value {
        let step_spans: Vec<serde_json::Value> = pipeline
            .steps
            .iter()
            .map(|s| self.step_to_span_json(s))
            .collect();

        serde_json::json!({
            "span_id": pipeline.span_id,
            "parent_span_id": pipeline.parent_span_id,
            "name": format!("pipeline.{}", pipeline.name),
            "start_time": pipeline.start_time.to_rfc3339(),
            "end_time": pipeline.end_time.map(|t| t.to_rfc3339()),
            "duration_ms": pipeline.duration_ms,
            "status": match pipeline.status {
                PipelineStatus::Completed => "ok",
                _ => "error"
            },
            "children": step_spans
        })
    }

    /// Convert step to span JSON.
    pub fn step_to_span_json(&self, step: &PipelineStep) -> serde_json::Value {
        serde_json::json!({
            "span_id": step.span_id,
            "parent_span_id": step.parent_span_id,
            "name": format!("step.{}", step.name),
            "start_time": step.start_time.to_rfc3339(),
            "end_time": step.end_time.map(|t| t.to_rfc3339()),
            "duration_ms": step.duration_ms,
            "status": match step.status {
                StepStatus::Completed => "ok",
                _ => "error"
            },
            "attributes": {
                "step.type": format!("{:?}", step.step_type),
                "step.model": step.model,
                "step.provider": step.provider
            },
            "token_usage": step.token_usage.as_ref().map(|u| serde_json::json!({
                "prompt_tokens": u.prompt_tokens,
                "completion_tokens": u.completion_tokens,
                "total_tokens": u.total_tokens
            }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_adapter_creation() {
        let adapter = OrchestratorAdapter::new("orchestrator-1");
        assert_eq!(adapter.orchestrator_id().as_str(), "orchestrator-1");
    }

    #[test]
    fn test_parse_workflow_telemetry() {
        let mut adapter = OrchestratorAdapter::new("orchestrator-1");

        let json_data = serde_json::json!({
            "workflow_id": "wf-123",
            "name": "document-processing",
            "status": "completed",
            "duration_ms": 5000,
            "trace_id": "trace-abc",
            "pipelines": [
                {
                    "pipeline_id": "pl-1",
                    "name": "extract",
                    "status": "completed",
                    "duration_ms": 2000,
                    "steps": [
                        {
                            "step_id": "step-1",
                            "name": "llm-extract",
                            "step_type": "llm_completion",
                            "status": "completed",
                            "model": "gpt-4",
                            "provider": "openai",
                            "token_usage": {
                                "prompt_tokens": 1000,
                                "completion_tokens": 500,
                                "total_tokens": 1500
                            }
                        }
                    ]
                }
            ]
        });

        let workflow = adapter.parse_workflow_telemetry(&json_data);
        assert!(workflow.is_ok());

        let workflow = workflow.unwrap();
        assert_eq!(workflow.workflow_id.as_str(), "wf-123");
        assert_eq!(workflow.name, "document-processing");
        assert_eq!(workflow.status, WorkflowStatus::Completed);
        assert_eq!(workflow.pipelines.len(), 1);
        assert_eq!(workflow.pipelines[0].steps.len(), 1);
    }

    #[test]
    fn test_token_usage_aggregation() {
        let mut adapter = OrchestratorAdapter::new("orchestrator-1");

        let json_data = serde_json::json!({
            "workflow_id": "wf-123",
            "name": "test-workflow",
            "status": "completed",
            "pipelines": [
                {
                    "pipeline_id": "pl-1",
                    "name": "pipeline-1",
                    "status": "completed",
                    "steps": [
                        {
                            "step_type": "llm_completion",
                            "name": "step-1",
                            "status": "completed",
                            "model": "gpt-4",
                            "token_usage": {
                                "prompt_tokens": 100,
                                "completion_tokens": 200,
                                "total_tokens": 300
                            }
                        },
                        {
                            "step_type": "llm_chat",
                            "name": "step-2",
                            "status": "completed",
                            "model": "gpt-4",
                            "token_usage": {
                                "prompt_tokens": 150,
                                "completion_tokens": 250,
                                "total_tokens": 400
                            }
                        }
                    ]
                }
            ]
        });

        let workflow = adapter.parse_workflow_telemetry(&json_data).unwrap();
        let usage = workflow.total_token_usage.unwrap();

        assert_eq!(usage.total_prompt_tokens, 250);
        assert_eq!(usage.total_completion_tokens, 450);
        assert_eq!(usage.total_tokens, 700);
    }

    #[test]
    fn test_should_sample_workflow() {
        let adapter = OrchestratorAdapter::new("orchestrator-1");

        // Failed workflow should be sampled
        let failed = WorkflowTelemetry {
            workflow_id: WorkflowId::new("wf-1"),
            name: "test".to_string(),
            orchestrator_id: OrchestratorId::new("orch-1"),
            trace_id: None,
            version: None,
            start_time: Utc::now(),
            end_time: None,
            duration_ms: Some(1000),
            status: WorkflowStatus::Failed,
            pipelines: Vec::new(),
            total_token_usage: None,
            total_cost_usd: None,
            input_params: HashMap::new(),
            output_results: HashMap::new(),
            metadata: HashMap::new(),
        };
        assert!(adapter.should_sample_workflow(&failed));

        // Slow workflow should be sampled
        let slow = WorkflowTelemetry {
            status: WorkflowStatus::Completed,
            duration_ms: Some(60000),
            ..failed.clone()
        };
        assert!(adapter.should_sample_workflow(&slow));

        // High cost workflow should be sampled
        let expensive = WorkflowTelemetry {
            status: WorkflowStatus::Completed,
            duration_ms: Some(1000),
            total_cost_usd: Some(5.0),
            ..failed.clone()
        };
        assert!(adapter.should_sample_workflow(&expensive));

        // Normal workflow should not be sampled
        let normal = WorkflowTelemetry {
            status: WorkflowStatus::Completed,
            duration_ms: Some(1000),
            total_cost_usd: Some(0.01),
            total_token_usage: Some(WorkflowTokenUsage {
                total_tokens: 1000,
                ..Default::default()
            }),
            ..failed
        };
        assert!(!adapter.should_sample_workflow(&normal));
    }

    #[test]
    fn test_stats_tracking() {
        let mut adapter = OrchestratorAdapter::new("orchestrator-1");

        for i in 0..3 {
            let json_data = serde_json::json!({
                "workflow_id": format!("wf-{}", i),
                "name": "test-workflow",
                "status": if i == 2 { "failed" } else { "completed" },
                "duration_ms": 1000 + i * 500,
                "pipelines": [
                    {
                        "pipeline_id": format!("pl-{}", i),
                        "name": "pipeline",
                        "status": "completed",
                        "steps": [
                            {
                                "step_type": "llm_completion",
                                "name": "step",
                                "status": "completed"
                            }
                        ]
                    }
                ]
            });
            adapter.parse_workflow_telemetry(&json_data).unwrap();
        }

        let stats = adapter.stats();
        assert_eq!(stats.total_workflows, 3);
        assert_eq!(stats.completed_workflows, 2);
        assert_eq!(stats.failed_workflows, 1);
        assert_eq!(stats.total_pipelines, 3);
        assert_eq!(stats.total_steps, 3);
        assert_eq!(stats.total_llm_calls, 3);
    }

    #[test]
    fn test_llm_steps_extraction() {
        let mut adapter = OrchestratorAdapter::new("orchestrator-1");

        let json_data = serde_json::json!({
            "workflow_id": "wf-1",
            "name": "test",
            "status": "completed",
            "pipelines": [
                {
                    "pipeline_id": "pl-1",
                    "name": "pipeline",
                    "status": "completed",
                    "steps": [
                        { "step_type": "llm_completion", "name": "s1", "status": "completed" },
                        { "step_type": "transform", "name": "s2", "status": "completed" },
                        { "step_type": "llm_chat", "name": "s3", "status": "completed" },
                        { "step_type": "api_call", "name": "s4", "status": "completed" }
                    ]
                }
            ]
        });

        adapter.parse_workflow_telemetry(&json_data).unwrap();

        let llm_steps = adapter.llm_steps();
        assert_eq!(llm_steps.len(), 2);
    }

    #[test]
    fn test_workflow_to_span_json() {
        let adapter = OrchestratorAdapter::new("orchestrator-1");

        let workflow = WorkflowTelemetry {
            workflow_id: WorkflowId::new("wf-123"),
            name: "test-workflow".to_string(),
            orchestrator_id: OrchestratorId::new("orch-1"),
            trace_id: Some("trace-abc".to_string()),
            version: Some("1.0.0".to_string()),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: Some(5000),
            status: WorkflowStatus::Completed,
            pipelines: Vec::new(),
            total_token_usage: Some(WorkflowTokenUsage {
                total_tokens: 1000,
                ..Default::default()
            }),
            total_cost_usd: Some(0.05),
            input_params: HashMap::new(),
            output_results: HashMap::new(),
            metadata: HashMap::new(),
        };

        let json = adapter.workflow_to_span_json(&workflow);
        assert_eq!(json["name"], "workflow.test-workflow");
        assert_eq!(json["duration_ms"], 5000);
        assert_eq!(json["status"], "ok");
    }

    #[test]
    fn test_clear() {
        let mut adapter = OrchestratorAdapter::new("orchestrator-1");

        let json_data = serde_json::json!({
            "workflow_id": "wf-1",
            "name": "test",
            "status": "completed",
            "pipelines": []
        });
        adapter.parse_workflow_telemetry(&json_data).unwrap();

        assert!(!adapter.workflows().is_empty());

        adapter.clear();

        assert!(adapter.workflows().is_empty());
        assert_eq!(adapter.stats().total_workflows, 0);
    }
}
