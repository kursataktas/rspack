mod chrome;
mod opentelemetry;
mod stdout;
mod tokio_console;
mod tracer;

use std::{fmt::Display, fs, io, path::Path, str::FromStr as _};

pub use chrome::ChromeTracer;
pub use opentelemetry::OtelTracer;
pub use stdout::StdoutTracer;
pub use tokio_console::TokioConsoleTracer;
pub use tracer::Tracer;
use tracing::Level;
use tracing_subscriber::{fmt::writer::BoxMakeWriter, EnvFilter, Layer};

pub mod otel {
  pub use opentelemetry;
  pub use opentelemetry_sdk as sdk;
  pub use tracing_opentelemetry as tracing;
}

pub fn generate_common_layers(
  filter: &str,
) -> Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync> {
  if let Some(default_level) = Level::from_str(filter).ok() {
    eprintln!(" - tracing filter: {}", filter);
    tracing_subscriber::filter::Targets::new()
      .with_target("rspack_core", default_level)
      .with_target("node_binding", default_level)
      .with_target("rspack_loader_swc", default_level)
      .with_target("rspack_loader_runner", default_level)
      .with_target("rspack_plugin_javascript", default_level)
      .with_target("rspack_resolver", Level::WARN)
      .boxed()
  } else {
    eprintln!(" - env filter: {}", filter);
    // SAFETY: we know that trace_var is `Ok(String)` now,
    // for the second unwrap, if we can't parse the directive, then the tracing result would be
    // unexpected, then panic is reasonable
    Box::new(EnvFilter::builder()
      .with_regex(true)
      .parse(filter)
      .expect("Parse tracing directive syntax failed,for details about the directive syntax you could refer https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives"))
  }
}

pub(crate) enum TraceWriter<'a> {
  Stdout,
  Stderr,
  File { path: &'a Path },
}

impl<'a> From<&'a str> for TraceWriter<'a> {
  fn from(s: &'a str) -> Self {
    match s {
      "stdout" => Self::Stdout,
      "stderr" => Self::Stderr,
      path => Self::File {
        path: Path::new(path),
      },
    }
  }
}

impl TraceWriter<'_> {
  pub fn make_writer(&self) -> BoxMakeWriter {
    match self {
      TraceWriter::Stdout => BoxMakeWriter::new(io::stdout),
      TraceWriter::Stderr => BoxMakeWriter::new(io::stderr),
      TraceWriter::File { path } => {
        BoxMakeWriter::new(fs::File::create(path).expect("Failed to create trace file"))
      }
    }
  }

  pub fn writer(&self) -> Box<dyn io::Write + Send + 'static> {
    match self {
      TraceWriter::Stdout => Box::new(io::stdout()),
      TraceWriter::Stderr => Box::new(io::stderr()),
      TraceWriter::File { path } => {
        Box::new(fs::File::create(path).expect("Failed to create trace file"))
      }
    }
  }

  pub fn display<'a>(&'a self) -> Box<dyn Display + 'a> {
    match self {
      TraceWriter::Stdout => Box::new("stdout"),
      TraceWriter::Stderr => Box::new("stderr"),
      TraceWriter::File { path } => Box::new(path.display()),
    }
  }
}
