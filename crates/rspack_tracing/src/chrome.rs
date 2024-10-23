use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::{
  layer::{Filter, Layer, SubscriberExt as _},
  util::SubscriberInitExt as _,
};

use crate::{
  tracer::{FilterLayers, Tracer},
  TraceWriter,
};

#[derive(Default)]
pub struct ChromeTracer {
  guard: Option<FlushGuard>,
}

impl Tracer for ChromeTracer {
  fn setup(&mut self, filter_layers: FilterLayers, output: &str) {
    let console_layer = console_subscriber::ConsoleLayer::builder().spawn();
    let trace_writer = TraceWriter::from(output);
    eprintln!(" - output: {}", trace_writer.display());
    let (chrome_layer, guard) = ChromeLayerBuilder::new()
      .include_args(true)
      .writer(trace_writer.writer())
      .build();
    self.guard = Some(guard);
    tracing_subscriber::registry()
      .with(filter_layers)
      .with(chrome_layer.with_filter(FilterEvent {}))
      .with(console_layer)
      .init();
  }

  fn teardown(&mut self) {
    if let Some(guard) = self.guard.take() {
      guard.flush();
    }
  }
}

// skip event because it's not useful for performance analysis
struct FilterEvent;

impl<S> Filter<S> for FilterEvent {
  fn enabled(
    &self,
    meta: &tracing::Metadata<'_>,
    _cx: &tracing_subscriber::layer::Context<'_, S>,
  ) -> bool {
    !meta.is_event()
  }
}
