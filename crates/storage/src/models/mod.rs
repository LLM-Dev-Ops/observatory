//! Data models for storage entities.
//!
//! This module contains the data models that represent database entities
//! for traces, metrics, and logs.

pub mod log;
pub mod metric;
pub mod trace;

// Re-exports
pub use log::{LogLevel, LogRecord};
pub use metric::{Metric, MetricDataPoint, MetricType};
pub use trace::{Trace, TraceEvent, TraceSpan};
