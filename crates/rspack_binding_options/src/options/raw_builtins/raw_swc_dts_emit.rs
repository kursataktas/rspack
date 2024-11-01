use napi_derive::napi;
use rspack_error::Result;
use rspack_loader_swc::SwcDtsEmitOptions;

#[napi(object, object_to_js = false)]
pub struct RawSwcDtsEmitRspackPluginOptions {
  pub root_dir: Option<String>,
  pub out_dir: Option<String>,
  pub include: Option<String>,
  pub mode: Option<String>,
}

impl TryFrom<RawSwcDtsEmitRspackPluginOptions> for SwcDtsEmitOptions {
  type Error = rspack_error::Error;

  fn try_from(value: RawSwcDtsEmitRspackPluginOptions) -> Result<Self> {
    Ok(SwcDtsEmitOptions {
      root_dir: value
        .root_dir
        .ok_or(rspack_error::error!("Failed to get 'root_dir'"))?,
      out_dir: value
        .out_dir
        .ok_or(rspack_error::error!("Failed to get 'out_dir'"))?,
      include: value
        .include
        .ok_or(rspack_error::error!("Failed to get 'include'"))?,
      mode: value
        .mode
        .ok_or(rspack_error::error!("Failed to get 'mode'"))?,
    })
  }
}
