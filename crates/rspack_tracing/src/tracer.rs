use tracing_subscriber::{Layer, Registry};

pub type FilterLayers = Box<dyn Layer<Registry> + Send + Sync>;

pub trait Tracer {
  fn setup(&mut self, filter_layers: FilterLayers, output: &str);
  fn teardown(&mut self);
}
