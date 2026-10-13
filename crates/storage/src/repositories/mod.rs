//! Repository layer for querying stored data.
//!
//! This module provides repository interfaces for querying traces, metrics,
//! and logs from the database.

pub mod instrumented;
pub mod log;
pub mod metric;
pub mod trace;

// Re-exports
pub use instrumented::{
    InstrumentedLogRepository, InstrumentedMetricRepository, InstrumentedTraceRepository,
};
pub use log::LogRepository;
pub use metric::MetricRepository;
pub use trace::TraceRepository;
