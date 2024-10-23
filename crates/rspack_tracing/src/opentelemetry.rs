use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_sdk::{runtime, Resource};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::tracer::Tracer;
use crate::tracer::FilterLayers;

pub struct OtelTracer {
  provider: opentelemetry_sdk::trace::TracerProvider,
}

impl Default for OtelTracer {
  fn default() -> Self {
    Self::new()
  }
}

impl OtelTracer {
  fn new() -> Self {
    let provider =
      opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(
          Resource::new(vec![KeyValue::new("service.name", "rspack-app")]),
        ))
        .install_batch(runtime::Tokio)
        .unwrap();
    Self { provider }
  }
}

impl Tracer for OtelTracer {
  fn setup(&mut self, filter_layers: FilterLayers, _output: &str) {
    global::set_tracer_provider(self.provider.clone());
    let trace = self.provider.tracer("rspack-app");
    tracing_subscriber::registry()
      .with(filter_layers)
      .with(OpenTelemetryLayer::new(trace))
      .init();
  }

  fn teardown(&mut self) {
    let _ = self.provider.shutdown();
    opentelemetry::global::shutdown_tracer_provider();
  }
}
