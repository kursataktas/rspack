use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use glob::glob_with;
use glob::{MatchOptions, Pattern as GlobPattern};
use rspack_core::{
  rspack_sources::RawSource, AssetInfo, Chunk, Compilation, CompilationAsset,
  CompilationFinishModules, CompilationProcessAssets, Logger, Plugin, PluginContext,
};
use rspack_error::Result;
use rspack_hook::{plugin, plugin_hook};
use rspack_paths::{AssertUtf8, Utf8Path, Utf8PathBuf};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

#[derive(Debug)]
pub struct SwcDtsEmitOptions {
  pub root_dir: String,
  pub out_dir: String,
  pub include: String,
  pub mode: String,
}

#[plugin]
#[derive(Debug)]
pub struct PluginSwcDtsEmit {
  pub(crate) options: Arc<SwcDtsEmitOptions>,
  pub(crate) dts_outputs: Arc<Mutex<HashMap<String, String>>>,
}

impl Eq for PluginSwcDtsEmit {}

impl PartialEq for PluginSwcDtsEmit {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.options, &other.options)
  }
}

const PLUGIN_NAME: &str = "rspack.SwcDtsEmitPlugin";

impl PluginSwcDtsEmit {
  pub fn new(options: SwcDtsEmitOptions) -> Self {
    Self::new_inner(Arc::new(options), Arc::new(Mutex::new(HashMap::default())))
  }
}

#[plugin_hook(CompilationFinishModules for PluginSwcDtsEmit)]
async fn finish_modules(&self, compilation: &mut Compilation) -> Result<()> {
  let module_graph = compilation.get_module_graph();
  let mut dts_outputs = self.dts_outputs.lock().expect("error in dts_outputs");

  for (_, a) in module_graph.modules() {
    let meta = &a.build_info().expect("parse_meta").parse_meta;
    let meta = meta.clone();

    dts_outputs.extend(meta);
  }
  Ok(())
}

#[plugin_hook(CompilationProcessAssets for PluginSwcDtsEmit, stage = Compilation::PROCESS_ASSETS_STAGE_DERIVED)]
async fn process_assets(&self, compilation: &mut Compilation) -> Result<()> {
  let mode = self.options.mode.clone();
  let logger = compilation.get_logger("rspack.SwcDtsEmitRspackPlugin");
  let start = logger.time("run dts emit");

  let dts_outputs = self.dts_outputs.lock().expect("error in dts_outputs");

  let mut root_dir = self.options.root_dir.clone();
  root_dir.push('/');

  let out_dir = self.options.out_dir.clone();

  if mode != "plugin" {
    for (key, source) in dts_outputs.iter() {
      let key = key.to_string();
      let path = key.strip_prefix("swc-dts-emit-plugin");

      let Some(source_filename) = path else {
        continue;
      };

      let Some(output_relative_path) = source_filename.strip_prefix(&root_dir) else {
        continue;
      };

      let output_relative_path = Utf8PathBuf::from(output_relative_path);

      let output_filename = Utf8PathBuf::from(out_dir.clone()).join(output_relative_path.clone());

      dbg!(&output_filename, &source, &output_relative_path);

      let asset_info = AssetInfo {
        source_filename: Some(source_filename.to_string()),
        ..Default::default()
      };

      compilation.emit_asset(
        output_filename.with_extension("d.ts").to_string(),
        CompilationAsset {
          source: Some(Arc::new(RawSource::from(source.as_str()))),
          info: asset_info,
        },
      );
    }

    logger.time_end(start);
    return Ok(());
  }

  // let context = compilation.options.context.as_path();

  // let output_dir = context.join(&self.options.out_dir);
  // let root_dir = context.join(&self.options.root_dir);

  // let include = self.options.include;

  // let include = if !include.contains('*') {
  //   let mut escaped = Utf8PathBuf::from(GlobPattern::escape(root_dir.as_str()));
  //   escaped.push("/**/*");
  //   escaped.as_str().to_string()
  // } else {
  //   include
  // };

  // let need_transform_files = glob_with(
  //   &include,
  //   // TODO: matchOptions
  //   MatchOptions {
  //     case_sensitive: true,
  //     require_literal_separator: Default::default(),
  //     require_literal_leading_dot: false,
  //   },
  // )
  // .expect("glob failed");

  // need_transform_files.into_iter().for_each(|item| {
  //   let source_filename =
  //     Utf8PathBuf::from_path_buf(item.expect("wrong glob result")).expect("from utf8 error");
  //   let output_relative_path =
  //     Utf8PathBuf::from(source_filename.strip_prefix(root_dir).expect("not in root"));
  //   let output_filename = Utf8PathBuf::from(root_dir).join(output_relative_path);

  //   let source = source_filename;

  //   let mut asset_info = AssetInfo {
  //     source_filename: Some(source_filename.to_string()),
  //     ..Default::default()
  //   };

  // compilation.emit_asset(
  //   output_filename.to_string(),
  //   CompilationAsset {
  //     source: Some(Arc::new(result.source)),
  //     info: asset_info,
  //   },
  // );
  // });

  Ok(())
}

// #[plugin_hook(CompilerCompilation for PluginSwcDtsEmit)]
// async fn compilation(
//   &self,
//   compilation: &mut Compilation,
//   params: &mut CompilationParams,
// ) -> Result<()> {
//   Ok(())
// }

impl Plugin for PluginSwcDtsEmit {
  fn name(&self) -> &'static str {
    PLUGIN_NAME
  }

  fn apply(
    &self,
    ctx: PluginContext<&mut rspack_core::ApplyContext>,
    _options: &rspack_core::CompilerOptions,
  ) -> Result<()> {
    // ctx
    //   .context
    //   .compiler_hooks
    //   .compilation
    //   .tap(compilation::new(self));

    ctx
      .context
      .compilation_hooks
      .finish_modules
      .tap(finish_modules::new(self));

    ctx
      .context
      .compilation_hooks
      .process_assets
      .tap(process_assets::new(self));

    Ok(())
  }
}
