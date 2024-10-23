use tracing_subscriber::fmt::format::FmtSpan;

use crate::{
  tracer::{FilterLayers, Tracer},
  TraceWriter,
};

pub struct StdoutTracer;

impl Tracer for StdoutTracer {
  fn setup(&mut self, filter_layers: FilterLayers, output: &str) {
    use tracing_subscriber::{fmt, prelude::*};
    let trace_writer = TraceWriter::from(output);

    tracing_subscriber::registry()
      .with(filter_layers)
      .with(
        fmt::layer()
          .pretty()
          .with_file(true)
          // To keep track of the closing point of spans
          .with_span_events(FmtSpan::CLOSE)
          .with_writer(trace_writer.make_writer()),
      )
      .init();
  }

  fn teardown(&mut self) {
    // noop
  }
}
