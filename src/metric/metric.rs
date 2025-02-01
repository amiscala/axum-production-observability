use opentelemetry::global;
use opentelemetry_sdk::metrics::{
    MeterProviderBuilder, MetricError, PeriodicReader, SdkMeterProvider,
};
use opentelemetry_sdk::runtime;

use crate::common::resource;

pub fn init_meter_provider() -> Result<SdkMeterProvider, MetricError> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()?;

    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    // For debugging in development
    let stdout_reader = PeriodicReader::builder(
        opentelemetry_stdout::MetricExporter::default(),
        runtime::Tokio,
    )
    .build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource())
        .with_reader(reader)
        .with_reader(stdout_reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}
