use std::{
  fs::File,
  hash::Hasher,
  io::{BufRead, BufReader, BufWriter, Read, Write},
  ops::Deref,
  path::PathBuf,
  sync::Mutex,
};

use futures::{executor::block_on, future::join_all};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rspack_error::{error, Result};
use rustc_hash::{FxHashMap as HashMap, FxHasher};

use super::{save_scope, Pack, PackContents, PackScope, PackStorageOptions};
use crate::Storage;

#[derive(Debug)]
pub struct PackStorage {
  options: PackStorageOptions,
  scopes: Mutex<HashMap<&'static str, PackScope>>,
  socpe_updates: Mutex<HashMap<&'static str, HashMap<Vec<u8>, Option<Vec<u8>>>>>,
}

impl PackStorage {
  pub fn new(options: PackStorageOptions) -> Self {
    Self {
      options,
      scopes: Default::default(),
      socpe_updates: Default::default(),
    }
  }
}

impl Storage for PackStorage {
  fn get_all(&self, name: &'static str) -> Result<Vec<&(Vec<u8>, Vec<u8>)>> {
    let mut scopes = self.scopes.lock().unwrap();
    let scope = scopes
      .entry(name)
      .or_insert_with(|| PackScope::new(self.options.location.join(name)));

    let is_valid = scope.validate(&self.options)?;
    if is_valid {
      scope.get_contents()
    } else {
      Err(error!("scope is inconsistent"))
    }
  }
  fn set(&self, scope: &'static str, key: Vec<u8>, value: Vec<u8>) {
    let mut inner = self.socpe_updates.lock().unwrap();
    let scope_map = inner.entry(scope).or_default();
    scope_map.insert(key, Some(value));
  }
  fn remove(&self, scope: &'static str, key: &[u8]) {
    let mut inner = self.socpe_updates.lock().unwrap();
    let scope_map = inner.entry(scope).or_default();
    scope_map.insert(key.to_vec(), None);
  }
  fn idle(&self) -> Result<()> {
    let options = self.options.clone();
    let data = std::mem::replace(&mut *self.socpe_updates.lock().unwrap(), Default::default());
    let mut scopes = std::mem::replace(&mut *self.scopes.lock().unwrap(), Default::default());
    for (scope_name, _) in &data {
      scopes
        .entry(scope_name)
        .or_insert_with(|| PackScope::new(self.options.location.join(scope_name)));
    }

    let new_scopes = save(scopes, data, options)?;
    std::mem::replace(&mut *self.scopes.lock().unwrap(), new_scopes);
    Ok(())
  }
}

fn save(
  scopes: HashMap<&'static str, PackScope>,
  data: HashMap<&'static str, HashMap<Vec<u8>, Option<Vec<u8>>>>,
  options: PackStorageOptions,
) -> Result<HashMap<&'static str, PackScope>> {
  let scopes = data
    .into_par_iter()
    .map(|(scope_name, map)| {
      let scope = scopes.get(&scope_name).unwrap_or_else(|| unreachable!());
      save_scope(scope, &map, &options)
    })
    .collect::<Result<Vec<_>>>()?;

  Ok(())
}
