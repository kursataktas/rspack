use crate::{tracer::FilterLayers, Tracer};

pub struct TokioConsoleTracer;

impl Tracer for TokioConsoleTracer {
  fn setup(&mut self, _filter: FilterLayers, _output: &str) {
    console_subscriber::init()
  }

  fn teardown(&mut self) {}
}
