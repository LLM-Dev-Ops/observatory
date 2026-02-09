// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Agentic execution context types for the Foundational Execution Unit.
//!
//! This module provides types for tracking agentic execution flow, orthogonal
//! to the existing OpenTelemetry-based LLM tracing. While [`crate::span::LlmSpan`]
//! tracks LLM model calls (tokens, costs, latency), the types here track which
//! repository is running, which agents are doing work, and what artifacts they produce.
//!
//! # Invariants
//!
//! After instrumentation, the following must always be true:
//!
//! ```text
//! Core
//!   └─ Repo (this repo)
//!       └─ Agent (one or more)
//! ```
//!
//! If no agent span exists, execution is INVALID.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an execution (top-level orchestration unit).
pub type ExecutionId = String;

/// Unique identifier for an execution span (repo-level or agent-level).
pub type ExecutionSpanId = String;

/// HTTP header names for execution context propagation.
///
/// These use a distinct prefix from W3C trace context to avoid collision
/// with existing OpenTelemetry tracing.
pub mod headers {
    /// Header carrying the execution ID.
    pub const X_EXECUTION_ID: &str = "x-execution-id";
    /// Header carrying the parent span ID from the caller.
    pub const X_EXECUTION_PARENT_SPAN_ID: &str = "x-execution-parent-span-id";
    /// Header carrying the repo name (optional, can also be configured server-side).
    pub const X_EXECUTION_REPO_NAME: &str = "x-execution-repo-name";
}

/// Discriminates repo-level vs agent-level spans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionSpanKind {
    /// Repo-level span: root of execution within this repository.
    Repo,
    /// Agent-level span: one agent performing work within the repo.
    Agent,
}

/// Status of an execution span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExecutionSpanStatus {
    /// Span is currently running.
    Running,
    /// Span completed successfully.
    Completed,
    /// Span failed.
    Failed,
    /// Span was cancelled/aborted.
    Cancelled,
}

impl Default for ExecutionSpanStatus {
    fn default() -> Self {
        ExecutionSpanStatus::Running
    }
}

/// Artifact content: either inline data or a reference URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "content_location", rename_all = "snake_case")]
pub enum ArtifactContent {
    /// Content is included inline (for small artifacts).
    Inline {
        /// Base64-encoded or raw content.
        data: String,
    },
    /// Content is stored externally.
    Reference {
        /// URI pointing to the stored content.
        uri: String,
    },
}

/// An artifact produced by an agent and attached to its span.
///
/// Artifacts have stable references via content-addressable SHA-256 hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique artifact ID (UUID v4).
    pub artifact_id: String,
    /// The execution span this artifact is attached to (must be an agent span).
    pub agent_span_id: ExecutionSpanId,
    /// Human-readable name (e.g., "analysis_report", "generated_code").
    pub name: String,
    /// MIME type of the artifact content.
    pub content_type: String,
    /// SHA-256 hash of the artifact content for stable referencing and integrity.
    pub content_hash: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Inline content or external reference.
    #[serde(flatten)]
    pub content: ArtifactContent,
    /// Timestamp when the artifact was created.
    pub created_at: DateTime<Utc>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A timestamped event within an execution span (append-only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    /// Event name.
    pub name: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Event attributes.
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

/// A single execution span within the agentic execution context.
///
/// This is orthogonal to [`crate::span::LlmSpan`] — `LlmSpan` tracks LLM calls,
/// `ExecutionSpan` tracks agentic execution flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpan {
    /// Unique span identifier (generated as UUID v4).
    pub span_id: ExecutionSpanId,
    /// The top-level execution ID from the calling agentics system.
    pub execution_id: ExecutionId,
    /// Parent span ID. For repo spans: the caller's span ID.
    /// For agent spans: the repo span ID. REQUIRED.
    pub parent_span_id: ExecutionSpanId,
    /// Whether this is a repo or agent span.
    pub kind: ExecutionSpanKind,
    /// Repository name.
    pub repo_name: String,
    /// Agent name (required for agent spans, None for repo spans).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// Span status.
    pub status: ExecutionSpanStatus,
    /// Start time.
    pub start_time: DateTime<Utc>,
    /// End time (set when span completes/fails).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in milliseconds (computed from start/end).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Artifacts attached to this span (only populated for agent spans).
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    /// Events recorded during this span (append-only log).
    #[serde(default)]
    pub events: Vec<ExecutionEvent>,
    /// Extensible attributes.
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
    /// Error message when status is Failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl ExecutionSpan {
    /// Create a new builder.
    pub fn builder() -> ExecutionSpanBuilder {
        ExecutionSpanBuilder::default()
    }

    /// Mark this span as completed, setting end_time and duration.
    pub fn complete(&mut self) {
        let now = Utc::now();
        self.end_time = Some(now);
        self.duration_ms = Some(
            now.signed_duration_since(self.start_time)
                .num_milliseconds()
                .unsigned_abs(),
        );
        self.status = ExecutionSpanStatus::Completed;
    }

    /// Mark this span as failed with an error message.
    pub fn fail(&mut self, error: impl Into<String>) {
        let now = Utc::now();
        self.end_time = Some(now);
        self.duration_ms = Some(
            now.signed_duration_since(self.start_time)
                .num_milliseconds()
                .unsigned_abs(),
        );
        self.status = ExecutionSpanStatus::Failed;
        self.error_message = Some(error.into());
    }

    /// Attach an artifact to this span.
    ///
    /// Returns `Err` if this is not an agent span.
    pub fn attach_artifact(&mut self, artifact: Artifact) -> crate::Result<()> {
        if self.kind != ExecutionSpanKind::Agent {
            return Err(crate::Error::invalid_input(
                "Artifacts can only be attached to agent spans",
            ));
        }
        self.artifacts.push(artifact);
        Ok(())
    }

    /// Record an event on this span.
    pub fn record_event(
        &mut self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) {
        self.events.push(ExecutionEvent {
            name: name.into(),
            timestamp: Utc::now(),
            attributes,
        });
    }

    /// Whether this span completed successfully.
    pub fn is_completed(&self) -> bool {
        self.status == ExecutionSpanStatus::Completed
    }

    /// Whether this span failed.
    pub fn is_failed(&self) -> bool {
        self.status == ExecutionSpanStatus::Failed
    }
}

/// Builder for [`ExecutionSpan`] instances.
#[derive(Default)]
pub struct ExecutionSpanBuilder {
    span_id: Option<ExecutionSpanId>,
    execution_id: Option<ExecutionId>,
    parent_span_id: Option<ExecutionSpanId>,
    kind: Option<ExecutionSpanKind>,
    repo_name: Option<String>,
    agent_name: Option<String>,
    status: ExecutionSpanStatus,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    artifacts: Vec<Artifact>,
    events: Vec<ExecutionEvent>,
    attributes: HashMap<String, serde_json::Value>,
    error_message: Option<String>,
}

impl ExecutionSpanBuilder {
    /// Set span ID. If not set, a UUID v4 will be generated.
    pub fn span_id(mut self, id: impl Into<ExecutionSpanId>) -> Self {
        self.span_id = Some(id.into());
        self
    }

    /// Set execution ID (required).
    pub fn execution_id(mut self, id: impl Into<ExecutionId>) -> Self {
        self.execution_id = Some(id.into());
        self
    }

    /// Set parent span ID (required).
    pub fn parent_span_id(mut self, id: impl Into<ExecutionSpanId>) -> Self {
        self.parent_span_id = Some(id.into());
        self
    }

    /// Set span kind: Repo or Agent (required).
    pub fn kind(mut self, kind: ExecutionSpanKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Set repository name (required).
    pub fn repo_name(mut self, name: impl Into<String>) -> Self {
        self.repo_name = Some(name.into());
        self
    }

    /// Set agent name (required for agent spans).
    pub fn agent_name(mut self, name: impl Into<String>) -> Self {
        self.agent_name = Some(name.into());
        self
    }

    /// Set status (default: Running).
    pub fn status(mut self, status: ExecutionSpanStatus) -> Self {
        self.status = status;
        self
    }

    /// Set start time. Defaults to `Utc::now()` if not set.
    pub fn start_time(mut self, time: DateTime<Utc>) -> Self {
        self.start_time = Some(time);
        self
    }

    /// Set end time.
    pub fn end_time(mut self, time: DateTime<Utc>) -> Self {
        self.end_time = Some(time);
        self
    }

    /// Add an artifact.
    pub fn artifact(mut self, artifact: Artifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    /// Add an event.
    pub fn event(mut self, event: ExecutionEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Add an attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Set error message (for failed spans).
    pub fn error_message(mut self, msg: impl Into<String>) -> Self {
        self.error_message = Some(msg.into());
        self
    }

    /// Build the [`ExecutionSpan`]. Returns `Err` if required fields are missing.
    pub fn build(self) -> crate::Result<ExecutionSpan> {
        let span_id = self
            .span_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let execution_id = self
            .execution_id
            .ok_or_else(|| crate::Error::invalid_input("execution_id is required"))?;
        let parent_span_id = self
            .parent_span_id
            .ok_or_else(|| crate::Error::invalid_input("parent_span_id is required"))?;
        let kind = self
            .kind
            .ok_or_else(|| crate::Error::invalid_input("kind is required"))?;
        let repo_name = self
            .repo_name
            .ok_or_else(|| crate::Error::invalid_input("repo_name is required"))?;
        let start_time = self.start_time.unwrap_or_else(Utc::now);

        if kind == ExecutionSpanKind::Agent && self.agent_name.is_none() {
            return Err(crate::Error::invalid_input(
                "agent_name is required for agent spans",
            ));
        }

        let duration_ms = self.end_time.map(|end| {
            end.signed_duration_since(start_time)
                .num_milliseconds()
                .unsigned_abs()
        });

        Ok(ExecutionSpan {
            span_id,
            execution_id,
            parent_span_id,
            kind,
            repo_name,
            agent_name: self.agent_name,
            status: self.status,
            start_time,
            end_time: self.end_time,
            duration_ms,
            artifacts: self.artifacts,
            events: self.events,
            attributes: self.attributes,
            error_message: self.error_message,
        })
    }
}

/// Execution context extracted from incoming request headers.
///
/// This is injected into request extensions by the execution middleware,
/// analogous to how `AuthContext` is injected by the auth middleware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// The top-level execution ID from the orchestration system.
    pub execution_id: ExecutionId,
    /// The caller's span ID (becomes `parent_span_id` for the repo span).
    pub parent_span_id: ExecutionSpanId,
    /// The repo-level span ID created on entry (populated by middleware).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_span_id: Option<ExecutionSpanId>,
    /// The repository name.
    pub repo_name: String,
}

/// The final output of an execution within this repository.
///
/// Contains the repo-level span, all nested agent spans, and all artifacts.
/// JSON-serializable, append-only, causally ordered via `parent_span_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The execution ID.
    pub execution_id: ExecutionId,
    /// The repo-level span.
    pub repo_span: ExecutionSpan,
    /// All agent-level spans, causally ordered by start_time.
    pub agent_spans: Vec<ExecutionSpan>,
    /// Whether the execution is considered valid.
    pub valid: bool,
    /// Validation errors (populated when `valid` is false).
    #[serde(default)]
    pub validation_errors: Vec<String>,
    /// Total artifacts across all agent spans.
    pub total_artifacts: usize,
    /// Total duration in milliseconds (repo span duration).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration_ms: Option<u64>,
}

impl ExecutionResult {
    /// Create a new result from a repo span and agent spans.
    pub fn new(repo_span: ExecutionSpan, agent_spans: Vec<ExecutionSpan>) -> Self {
        Self {
            execution_id: repo_span.execution_id.clone(),
            total_duration_ms: repo_span.duration_ms,
            total_artifacts: agent_spans.iter().map(|s| s.artifacts.len()).sum(),
            repo_span,
            agent_spans,
            valid: false,
            validation_errors: Vec::new(),
        }
    }

    /// Validate the execution result according to enforcement rules.
    ///
    /// Checks:
    /// - Repo span has a non-empty `parent_span_id`
    /// - At least one agent span was emitted
    /// - All agent spans reference the repo span as parent
    /// - No duplicate span IDs
    pub fn validate(mut self) -> Self {
        self.validation_errors.clear();

        // Rule: repo span must have a parent_span_id
        if self.repo_span.parent_span_id.is_empty() {
            self.validation_errors.push(
                "Repo span is missing parent_span_id from caller".to_string(),
            );
        }

        // Rule: must have at least one agent span
        if self.agent_spans.is_empty() {
            self.validation_errors.push(
                "No agent spans emitted -- execution has no evidence of agent work".to_string(),
            );
        }

        // Rule: every agent span must have parent_span_id == repo span_id
        for agent_span in &self.agent_spans {
            if agent_span.parent_span_id != self.repo_span.span_id {
                self.validation_errors.push(format!(
                    "Agent span {} has parent_span_id {} but expected repo span {}",
                    agent_span.span_id, agent_span.parent_span_id, self.repo_span.span_id
                ));
            }
        }

        // Rule: no two agent spans should share the same span_id
        let mut seen_ids = std::collections::HashSet::new();
        for agent_span in &self.agent_spans {
            if !seen_ids.insert(&agent_span.span_id) {
                self.validation_errors
                    .push(format!("Duplicate agent span_id: {}", agent_span.span_id));
            }
        }

        self.valid = self.validation_errors.is_empty();
        self.total_artifacts = self
            .agent_spans
            .iter()
            .map(|s| s.artifacts.len())
            .sum();
        self.total_duration_ms = self.repo_span.duration_ms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_repo_span(parent: &str) -> ExecutionSpan {
        ExecutionSpan::builder()
            .execution_id("exec-1")
            .parent_span_id(parent)
            .kind(ExecutionSpanKind::Repo)
            .repo_name("llm-observatory")
            .build()
            .unwrap()
    }

    fn make_agent_span(repo_span_id: &str) -> ExecutionSpan {
        ExecutionSpan::builder()
            .execution_id("exec-1")
            .parent_span_id(repo_span_id)
            .kind(ExecutionSpanKind::Agent)
            .repo_name("llm-observatory")
            .agent_name("test-agent")
            .build()
            .unwrap()
    }

    #[test]
    fn test_builder_generates_uuid_span_id() {
        let span = make_repo_span("parent-1");
        assert!(!span.span_id.is_empty());
        // Should be a valid UUID
        assert!(Uuid::parse_str(&span.span_id).is_ok());
    }

    #[test]
    fn test_builder_requires_execution_id() {
        let result = ExecutionSpan::builder()
            .parent_span_id("parent-1")
            .kind(ExecutionSpanKind::Repo)
            .repo_name("test")
            .build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("execution_id"));
    }

    #[test]
    fn test_builder_requires_parent_span_id() {
        let result = ExecutionSpan::builder()
            .execution_id("exec-1")
            .kind(ExecutionSpanKind::Repo)
            .repo_name("test")
            .build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parent_span_id"));
    }

    #[test]
    fn test_builder_requires_kind() {
        let result = ExecutionSpan::builder()
            .execution_id("exec-1")
            .parent_span_id("parent-1")
            .repo_name("test")
            .build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("kind"));
    }

    #[test]
    fn test_builder_requires_repo_name() {
        let result = ExecutionSpan::builder()
            .execution_id("exec-1")
            .parent_span_id("parent-1")
            .kind(ExecutionSpanKind::Repo)
            .build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("repo_name"));
    }

    #[test]
    fn test_agent_span_requires_agent_name() {
        let result = ExecutionSpan::builder()
            .execution_id("exec-1")
            .parent_span_id("parent-1")
            .kind(ExecutionSpanKind::Agent)
            .repo_name("test")
            .build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("agent_name"));
    }

    #[test]
    fn test_repo_span_does_not_require_agent_name() {
        let span = make_repo_span("parent-1");
        assert_eq!(span.kind, ExecutionSpanKind::Repo);
        assert!(span.agent_name.is_none());
    }

    #[test]
    fn test_span_complete() {
        let mut span = make_repo_span("parent-1");
        assert_eq!(span.status, ExecutionSpanStatus::Running);
        assert!(span.end_time.is_none());

        span.complete();
        assert_eq!(span.status, ExecutionSpanStatus::Completed);
        assert!(span.end_time.is_some());
        assert!(span.duration_ms.is_some());
    }

    #[test]
    fn test_span_fail() {
        let mut span = make_repo_span("parent-1");
        span.fail("something went wrong");
        assert_eq!(span.status, ExecutionSpanStatus::Failed);
        assert_eq!(
            span.error_message.as_deref(),
            Some("something went wrong")
        );
        assert!(span.end_time.is_some());
    }

    #[test]
    fn test_artifact_attachment_only_on_agent_spans() {
        let mut repo_span = make_repo_span("parent-1");
        let artifact = Artifact {
            artifact_id: Uuid::new_v4().to_string(),
            agent_span_id: "agent-1".to_string(),
            name: "test".to_string(),
            content_type: "text/plain".to_string(),
            content_hash: "abc123".to_string(),
            size_bytes: 5,
            content: ArtifactContent::Inline {
                data: "hello".to_string(),
            },
            created_at: Utc::now(),
            metadata: HashMap::new(),
        };

        // Should fail on repo span
        assert!(repo_span.attach_artifact(artifact.clone()).is_err());

        // Should succeed on agent span
        let mut agent_span = make_agent_span(&repo_span.span_id);
        assert!(agent_span.attach_artifact(artifact).is_ok());
        assert_eq!(agent_span.artifacts.len(), 1);
    }

    #[test]
    fn test_record_event() {
        let mut span = make_repo_span("parent-1");
        assert!(span.events.is_empty());

        span.record_event("started_processing", HashMap::new());
        assert_eq!(span.events.len(), 1);
        assert_eq!(span.events[0].name, "started_processing");
    }

    #[test]
    fn test_execution_result_valid() {
        let repo_span = make_repo_span("caller-span-1");
        let agent_span = make_agent_span(&repo_span.span_id);

        let result = ExecutionResult::new(repo_span, vec![agent_span]).validate();
        assert!(result.valid);
        assert!(result.validation_errors.is_empty());
    }

    #[test]
    fn test_execution_result_rejects_empty_parent_span_id() {
        let repo_span = make_repo_span("");
        let agent_span = make_agent_span(&repo_span.span_id);

        let result = ExecutionResult::new(repo_span, vec![agent_span]).validate();
        assert!(!result.valid);
        assert!(result
            .validation_errors
            .iter()
            .any(|e| e.contains("parent_span_id")));
    }

    #[test]
    fn test_execution_result_rejects_no_agent_spans() {
        let repo_span = make_repo_span("caller-span-1");

        let result = ExecutionResult::new(repo_span, vec![]).validate();
        assert!(!result.valid);
        assert!(result
            .validation_errors
            .iter()
            .any(|e| e.contains("No agent spans")));
    }

    #[test]
    fn test_execution_result_rejects_wrong_parent() {
        let repo_span = make_repo_span("caller-span-1");
        let mut agent_span = make_agent_span(&repo_span.span_id);
        agent_span.parent_span_id = "wrong-parent".to_string();

        let result = ExecutionResult::new(repo_span, vec![agent_span]).validate();
        assert!(!result.valid);
        assert!(result
            .validation_errors
            .iter()
            .any(|e| e.contains("wrong-parent")));
    }

    #[test]
    fn test_execution_result_rejects_duplicate_span_ids() {
        let repo_span = make_repo_span("caller-span-1");
        let agent1 = make_agent_span(&repo_span.span_id);
        let mut agent2 = make_agent_span(&repo_span.span_id);
        agent2.span_id = agent1.span_id.clone(); // Duplicate

        let result = ExecutionResult::new(repo_span, vec![agent1, agent2]).validate();
        assert!(!result.valid);
        assert!(result
            .validation_errors
            .iter()
            .any(|e| e.contains("Duplicate")));
    }

    #[test]
    fn test_execution_result_counts_artifacts() {
        let repo_span = make_repo_span("caller-span-1");
        let mut agent_span = make_agent_span(&repo_span.span_id);
        let artifact = Artifact {
            artifact_id: Uuid::new_v4().to_string(),
            agent_span_id: agent_span.span_id.clone(),
            name: "report".to_string(),
            content_type: "application/json".to_string(),
            content_hash: "deadbeef".to_string(),
            size_bytes: 42,
            content: ArtifactContent::Inline {
                data: "{}".to_string(),
            },
            created_at: Utc::now(),
            metadata: HashMap::new(),
        };
        agent_span.attach_artifact(artifact).unwrap();

        let result = ExecutionResult::new(repo_span, vec![agent_span]).validate();
        assert!(result.valid);
        assert_eq!(result.total_artifacts, 1);
    }

    #[test]
    fn test_span_serialization_roundtrip() {
        let span = make_repo_span("parent-1");
        let json = serde_json::to_string(&span).unwrap();
        let deserialized: ExecutionSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.span_id, span.span_id);
        assert_eq!(deserialized.execution_id, span.execution_id);
        assert_eq!(deserialized.kind, ExecutionSpanKind::Repo);
    }

    #[test]
    fn test_execution_result_serialization() {
        let repo_span = make_repo_span("caller-span-1");
        let agent_span = make_agent_span(&repo_span.span_id);
        let result = ExecutionResult::new(repo_span, vec![agent_span]).validate();

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.valid, result.valid);
        assert_eq!(deserialized.agent_spans.len(), 1);
    }

    #[test]
    fn test_default_status_is_running() {
        let span = make_repo_span("parent-1");
        assert_eq!(span.status, ExecutionSpanStatus::Running);
    }

    #[test]
    fn test_builder_with_explicit_span_id() {
        let span = ExecutionSpan::builder()
            .span_id("my-custom-id")
            .execution_id("exec-1")
            .parent_span_id("parent-1")
            .kind(ExecutionSpanKind::Repo)
            .repo_name("test")
            .build()
            .unwrap();
        assert_eq!(span.span_id, "my-custom-id");
    }

    #[test]
    fn test_builder_with_attributes() {
        let span = ExecutionSpan::builder()
            .execution_id("exec-1")
            .parent_span_id("parent-1")
            .kind(ExecutionSpanKind::Repo)
            .repo_name("test")
            .attribute("key", serde_json::json!("value"))
            .build()
            .unwrap();
        assert_eq!(
            span.attributes.get("key"),
            Some(&serde_json::json!("value"))
        );
    }
}
