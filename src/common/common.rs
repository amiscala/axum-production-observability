use crate::logging::init_logs;
use crate::metric::init_meter_provider;
use crate::tracing::init_tracer_provider;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::{
    attribute::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

pub fn resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "develop"),
        ],
        SCHEMA_URL,
    )
}

pub struct ObservabilityGuard {
    _tracer_provider: TracerProvider,
    _metric_provider: SdkMeterProvider,
    _logger_provider: LoggerProvider,
}

impl ObservabilityGuard {
    pub fn new(log_level: tracing_core::Level, tracer_name: &str,otlp_collector_url: &str) -> Self {
        let tracer_provider = init_tracer_provider(otlp_collector_url).expect("Error while constructing the tracer");
        global::set_tracer_provider(tracer_provider.clone());
        let tracer = tracer_provider.tracer(tracer_name.to_string());
        let logger_provider = init_logs(otlp_collector_url).expect("Error while constructing the logger");
        let layer = OpenTelemetryTracingBridge::new(&logger_provider).with_filter(tracing_subscriber::filter::filter_fn(|metadata| metadata.target().starts_with("log")));
        let metric_provider = init_meter_provider().expect("Error while constructing metric provider");
        global::set_meter_provider(metric_provider.clone());
        tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::from_level(
                log_level,
            ))
            // .with(tracing_subscriber::fmt::layer())
            .with(MetricsLayer::new(metric_provider.clone()))
            .with(OpenTelemetryLayer::new(tracer).with_filter(tracing_subscriber::filter::filter_fn(|metadata| !metadata.target().starts_with("log"))))
            .with(layer)
            .init();
        ObservabilityGuard {
            _tracer_provider: tracer_provider,
            _logger_provider: logger_provider,
            _metric_provider: metric_provider,
        }
    }
}
