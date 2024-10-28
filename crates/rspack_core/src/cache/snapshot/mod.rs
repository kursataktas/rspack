mod option;
mod strategy;

use std::path::PathBuf;

use rspack_cacheable::{from_bytes, to_bytes};
use rustc_hash::FxHashSet as HashSet;

pub use self::option::{PathMatcher, SnapshotOptions};
use self::strategy::{Strategy, StrategyHelper, ValidateResult};
use super::storage::ArcStorage;

#[derive(Debug)]
pub struct Snapshot {
  scope: String,
  storage: ArcStorage,
  options: SnapshotOptions,
}

impl Snapshot {
  pub fn new(scope_prefix: String, storage: ArcStorage, options: SnapshotOptions) -> Self {
    Self {
      scope: scope_prefix + "_snapshot",
      storage,
      options,
    }
  }

  pub fn add(&self, files: impl Iterator<Item = &PathBuf>) {
    let default_strategy = Strategy::CompileTime(StrategyHelper::compile_time());
    let mut helper = StrategyHelper::default();
    for path in files {
      if !path.exists() {
        continue;
      }
      let path_str = path.to_str().expect("should can convert to string");
      if self.options.is_immutable_path(path_str) {
        continue;
      }
      if self.options.is_managed_path(path_str) {
        if let Some(v) = helper.lib_version(&path) {
          self.storage.set(
            &self.scope,
            path_str.as_bytes().to_vec(),
            to_bytes::<_, ()>(&Strategy::LibVersion(v), &()).expect("should to bytes success"),
          );
        }
      }
      // compiler time
      self.storage.set(
        &self.scope,
        path_str.as_bytes().to_vec(),
        to_bytes::<_, ()>(&default_strategy, &()).expect("should to bytes success"),
      );
    }
  }

  pub fn remove(&self, files: impl Iterator<Item = &PathBuf>) {
    for item in files {
      self.storage.remove(
        &self.scope,
        item.to_str().expect("should have str").as_bytes(),
      )
    }
  }

  pub fn calc_modified_files(&self) -> (HashSet<PathBuf>, HashSet<PathBuf>) {
    let mut helper = StrategyHelper::default();
    let mut modified_files = HashSet::default();
    let mut deleted_files = HashSet::default();

    for (key, value) in self.storage.get_all(&self.scope) {
      let path = PathBuf::from(String::from_utf8(key).expect("should have utf8 key"));
      let strategy: Strategy =
        from_bytes::<Strategy, ()>(&value, &mut ()).expect("should from bytes success");
      match helper.validate(&path, &strategy) {
        ValidateResult::Modified => {
          modified_files.insert(path);
        }
        ValidateResult::Deleted => {
          deleted_files.insert(path);
        }
        _ => {}
      }
    }
    (modified_files, deleted_files)
  }
}
