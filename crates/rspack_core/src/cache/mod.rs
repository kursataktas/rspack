pub mod snapshot;
mod storage;

use std::{path::PathBuf, sync::Arc};

use self::{
  snapshot::Snapshot,
  storage::{ArcStorage, FsStorage},
};
use crate::CompilerOptions;

// TODO call write storage only build success
#[derive(Debug)]
pub struct Cache {
  storage: ArcStorage,
  snapshot: Snapshot,
}

// TODO conside multi compiler
impl Cache {
  pub fn new(compiler_option: Arc<CompilerOptions>) -> Self {
    let storage = Arc::new(FsStorage::new(
      PathBuf::from(compiler_option.context.as_str())
        .join("node_modules/.cache/rspack/compiler-id-version"),
    ));
    Self {
      snapshot: Snapshot::new(
        String::from("compilerId_childCompilerName"),
        storage.clone(),
        Default::default(),
      ),
      storage,
    }
  }

  pub fn idle(&self) {
    self.storage.idle();
  }
}
