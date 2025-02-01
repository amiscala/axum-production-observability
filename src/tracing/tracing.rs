use opentelemetry::global;
use opentelemetry::trace::TraceError;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace::TracerProvider;

use crate::common::resource;

pub fn init_tracer_provider(otlp_collector_endpoint: String) -> Result<TracerProvider, TraceError> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_collector_endpoint)
        .build()?;

    Ok(TracerProvider::builder()
        .with_resource(resource())
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_batch_exporter(
            opentelemetry_stdout::SpanExporter::default(),
            runtime::Tokio,
        )
        .build())
}
