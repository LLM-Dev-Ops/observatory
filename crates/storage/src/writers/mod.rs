//! Writer layer for batch insertion of data.
//!
//! This module provides efficient batch writers for inserting traces, metrics,
//! and logs into the database.
//!
//! Two write methods are available:
//! - **INSERT** (default): Standard batch INSERT using sqlx QueryBuilder
//! - **COPY**: PostgreSQL COPY protocol for 10-100x faster batch inserts

pub mod copy;
pub mod copy_instrumented;
pub mod instrumented;
pub mod log;
pub mod metric;
pub mod trace;

// Re-exports
pub use copy::CopyWriter;
pub use copy_instrumented::InstrumentedCopyWriter;
pub use instrumented::{InstrumentedLogWriter, InstrumentedMetricWriter, InstrumentedTraceWriter};
pub use log::LogWriter;
pub use metric::MetricWriter;
pub use trace::{TraceWriter, WriteMethod};
