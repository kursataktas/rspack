use cow_utils::CowUtils;
use rspack_collections::Identifier;
use rspack_core::{
  has_hash_placeholder, impl_runtime_module, Compilation, Filename, PublicPath, RuntimeModule,
};

#[impl_runtime_module]
#[derive(Debug)]
pub struct PublicPathRuntimeModule {
  id: Identifier,
  public_path: Box<Filename>,
}

impl PublicPathRuntimeModule {
  pub fn new(public_path: Box<Filename>) -> Self {
    Self::with_default(Identifier::from("webpack/runtime/public_path"), public_path)
  }

  fn generate(&self, compilation: &Compilation) -> rspack_error::Result<String> {
    Ok(
      include_str!("runtime/public_path.js")
        .cow_replace(
          "__PUBLIC_PATH_PLACEHOLDER__",
          &PublicPath::render_filename(compilation, &self.public_path),
        )
        .to_string(),
    )
  }
}

impl RuntimeModule for PublicPathRuntimeModule {
  fn name(&self) -> Identifier {
    self.id
  }

  // be cacheable only when the template does not contain a hash placeholder
  fn cacheable(&self) -> bool {
    if let Some(template) = self.public_path.template() {
      !has_hash_placeholder(template)
    } else {
      false
    }
  }

  fn dependent_hash(&self) -> bool {
    true
  }
}
