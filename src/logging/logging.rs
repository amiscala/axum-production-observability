use opentelemetry_otlp::{LogExporter, WithExportConfig};
use opentelemetry_sdk::logs::{LogError, LoggerProvider};
use opentelemetry_sdk::runtime;

use crate::common::resource;

pub fn init_logs(otlp_collector_address: &str) -> Result<LoggerProvider, LogError> {
    let console_exporter = opentelemetry_stdout::LogExporter::default();
    let exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_collector_address)
        .build()?;


    Ok(LoggerProvider::builder()
        .with_resource(resource())
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_batch_exporter(console_exporter, runtime::Tokio)
        .build())
}