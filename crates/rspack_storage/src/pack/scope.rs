use std::{
  hash::Hasher,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

use futures::{executor::block_on, future::join_all, TryFutureExt};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rspack_error::{error, miette::Error, Result};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet, FxHasher};

use super::{Pack, PackContentsState, PackFileMeta, PackStorageOptions, ScopeMeta};
use crate::pack::{PackKeys, PackKeysState};

#[derive(Debug, Default)]
pub enum ScopeMetaState {
  #[default]
  Pending,
  Failed(Error),
  Value(ScopeMeta),
}

#[derive(Debug, Default)]
pub enum ScopePacksState {
  #[default]
  Pending,
  Failed(Error),
  Value(HashMap<String, Pack>),
}

#[derive(Debug)]
pub struct PackScope {
  pub path: PathBuf,
  pub meta: ScopeMetaState,
  pub packs: ScopePacksState,
}

impl PackScope {
  pub fn new(path: PathBuf) -> Self {
    Self {
      path,
      meta: ScopeMetaState::Pending,
      packs: ScopePacksState::Pending,
    }
  }

  pub fn get_contents(&mut self) -> Result<Vec<&(Vec<u8>, Vec<u8>)>> {
    self.ensure_meta()?;
    self.ensure_pack_keys()?;
    self.ensure_pack_contents()?;

    if let ScopePacksState::Value(packs) = &self.packs {
      let mut res = vec![];
      for (_, pack) in packs {
        if let PackContentsState::Value(contents) = &pack.contents {
          res.extend(contents);
        }
      }
      Ok(res)
    } else {
      unreachable!()
    }
  }

  pub fn validate(&mut self, options: &PackStorageOptions) -> Result<bool> {
    self.ensure_meta()?;

    let ScopeMetaState::Value(meta) = &self.meta else {
      unreachable!()
    };

    // validate meta
    if meta.buckets != options.buckets || meta.max_pack_size != options.max_pack_size {
      return Err(error!("cache options changed"));
    }

    let current_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| error!("get current time failed: {}", e))?
      .as_secs();

    if current_time - meta.last_modified > options.expires {
      return Err(error!("cache meta expired"));
    }

    // validate packs
    self.ensure_pack_keys()?;
    let validate = self.validate_packs()?;

    Ok(validate)
  }

  fn validate_packs(&self) -> Result<bool> {
    let ScopePacksState::Value(packs) = &self.packs else {
      return Err(error!("packs not ready"));
    };

    async fn validate_pack(hash: String, file: PathBuf, keys: PackKeys) -> bool {
      match Pack::validate(&file, &keys, &hash) {
        Ok(v) => v,
        Err(_) => false,
      }
    }

    let tasks = packs
      .iter()
      .filter(|(_, pack)| matches!(pack.keys, PackKeysState::Value(_)))
      .map(|arg| {
        let PackKeysState::Value(keys) = &arg.1.keys else {
          unreachable!()
        };
        tokio::spawn(validate_pack(
          arg.0.to_owned(),
          arg.1.path.to_owned(),
          keys.to_owned(),
        ))
        .map_err(|e| error!("{}", e))
      });

    let pack_validates = block_on(join_all(tasks))
      .into_iter()
      .collect::<Result<Vec<bool>>>()?;

    Ok(pack_validates.iter().all(|v| *v))
  }

  fn ensure_pack_keys(&mut self) -> Result<()> {
    let ScopePacksState::Value(packs) = &mut self.packs else {
      return Err(error!("packs not ready"));
    };

    async fn load_pack_key(hash: String, file: PathBuf) -> (String, PackKeysState) {
      (
        hash,
        match Pack::read_keys(&file) {
          Ok(v) => PackKeysState::Value(v),
          Err(e) => PackKeysState::Failed(e),
        },
      )
    }

    let tasks = packs
      .iter()
      .filter(|(_, pack)| matches!(pack.keys, PackKeysState::Pending))
      .map(|arg| {
        tokio::spawn(load_pack_key(arg.0.to_owned(), arg.1.path.to_owned()))
          .map_err(|e| error!("{}", e))
      });

    let pack_keys = block_on(join_all(tasks))
      .into_iter()
      .collect::<Result<Vec<(String, PackKeysState)>>>()?;

    for (hash, item) in pack_keys {
      if let Some(pack) = packs.get_mut(&hash) {
        pack.keys = item
      }
    }

    Ok(())
  }

  fn ensure_pack_contents(&mut self) -> Result<()> {
    let ScopePacksState::Value(packs) = &mut self.packs else {
      return Err(error!("packs not ready"));
    };

    async fn load_pack_content(
      hash: String,
      file: PathBuf,
      keys: PackKeys,
    ) -> (String, PackContentsState) {
      (
        hash,
        match Pack::read_contents(&file, &keys) {
          Ok(v) => PackContentsState::Value(v),
          Err(e) => PackContentsState::Failed(e),
        },
      )
    }

    let tasks = packs
      .iter()
      .filter(|(_, pack)| {
        matches!(pack.contents, PackContentsState::Pending)
          && matches!(pack.keys, PackKeysState::Value(_))
      })
      .map(|arg| {
        let PackKeysState::Value(keys) = &arg.1.keys else {
          unreachable!()
        };
        tokio::spawn(load_pack_content(
          arg.0.to_owned(),
          arg.1.path.to_owned(),
          keys.to_owned(),
        ))
        .map_err(|e| error!("{}", e))
      });

    let pack_contents = block_on(join_all(tasks))
      .into_iter()
      .collect::<Result<Vec<(String, PackContentsState)>>>()?;

    for (hash, item) in pack_contents {
      if let Some(pack) = packs.get_mut(&hash) {
        pack.contents = item
      }
    }

    Ok(())
  }

  fn ensure_meta(&mut self) -> Result<()> {
    if matches!(self.meta, ScopeMetaState::Pending) {
      self.meta = match ScopeMeta::read(&self.path) {
        Ok(v) => ScopeMetaState::Value(v),
        Err(e) => ScopeMetaState::Failed(e),
      };
    }

    match &self.meta {
      ScopeMetaState::Pending => unreachable!(),
      ScopeMetaState::Failed(e) => {
        self.packs = ScopePacksState::Failed(error!("load scope meta failed"));
        return Err(error!("{}", e));
      }
      ScopeMetaState::Value(meta) => match &self.packs {
        ScopePacksState::Pending => {
          self.packs = ScopePacksState::Value(
            (0..meta.buckets)
              .into_iter()
              .map(|bucket_id| {
                let bucket_dir = self.path.join(bucket_id.to_string());
                meta
                  .packs
                  .get(bucket_id)
                  .expect("should have pack")
                  .iter()
                  .map(|pack_meta| {
                    (
                      pack_meta.hash.clone(),
                      Pack::new(bucket_dir.join(&pack_meta.name)),
                    )
                  })
                  .collect::<Vec<_>>()
              })
              .flatten()
              .collect::<HashMap<String, Pack>>(),
          );
          Ok(())
        }
        ScopePacksState::Failed(e) => {
          return Err(error!("{}", e));
        }
        ScopePacksState::Value(_) => Ok(()),
      },
    }
  }
}

pub fn save_scope(
  scope: &PackScope,
  data: &HashMap<Vec<u8>, Option<Vec<u8>>>,
  options: &PackStorageOptions,
) -> Result<PackScope> {
  let mut new_scope = PackScope::new(scope.path.clone());
  let mut new_scope_meta = ScopeMeta::new(options);
  new_scope_meta.packs = Vec::with_capacity(options.buckets);
  let mut new_scope_packs: HashMap<String, Pack> = HashMap::default();

  let scope_old_pack_metas = if let ScopeMetaState::Value(meta) = &scope.meta {
    meta.packs.to_owned()
  } else {
    vec![]
  };

  let scope_old_packs = if let ScopePacksState::Value(packs) = &scope.packs {
    packs.to_owned()
  } else {
    &HashMap::default()
  };

  let mut dirty_buckets: HashMap<usize, Vec<&Vec<u8>>> = HashMap::default();
  for key in data.keys() {
    dirty_buckets
      .entry(get_key_bucket_id(key, options.buckets))
      .or_default()
      .push(key);
  }

  for (dirty_bucket_id, dirty_keys) in dirty_buckets {
    let bucket_old_pack_metas = if let Some(pack_metas) = scope_old_pack_metas.get(dirty_bucket_id)
    {
      pack_metas.to_owned()
    } else {
      vec![]
    };

    let bucket_old_packs = bucket_old_pack_metas
      .iter()
      .map(|meta| {
        (
          meta.hash.clone(),
          scope_old_packs
            .get(&meta.hash)
            .expect("should have old pack"),
        )
      })
      .collect::<HashMap<String, &Pack>>();

    let key_to_pack_hash =
      bucket_old_packs
        .iter()
        .fold(HashMap::default(), |mut acc, (pack_hash, pack)| {
          let keys = if let PackKeysState::Value(keys) = &pack.keys {
            keys.to_owned()
          } else {
            vec![]
          };
          for key in keys {
            acc.insert(key, pack_hash);
          }
          acc
        });

    let mut insert_keys = HashSet::default();
    let mut removed_packs = HashSet::default();
    let mut removed_keys = HashSet::default();

    // let mut updated_packs = HashSet::default();
    // let mut updated_keys = HashSet::default();

    for data_key in dirty_keys {
      if data.get(data_key).expect("should have value").is_some() {
        if let Some(pack_hash) = key_to_pack_hash.get(data_key) {
          // update
          // updated_packs.insert(pack_hash);
          // updated_keys.insert(data_key)
          insert_keys.insert(data_key);
          removed_packs.insert(*pack_hash);
        } else {
          // insert
          insert_keys.insert(data_key);
        }
      } else {
        if let Some(pack_hash) = key_to_pack_hash.get(data_key) {
          // update
          removed_packs.insert(*pack_hash);
          removed_keys.insert(data_key);
        } else {
          // not exists, do nothing
        }
      }
    }
    // TODO: try to update pack

    // pull out removed packs
    let mut res = vec![];
    for pack_hash in removed_packs.clone() {
      let old_pack = scope_old_packs.get(pack_hash).expect("should have pack");
      if let PackContentsState::Value(contents) = &old_pack.contents {
        res.extend(contents.clone());
      }
    }
    res = res
      .into_iter()
      .filter(|(key, _)| !removed_keys.contains(key))
      .filter(|(key, _)| !insert_keys.contains(key))
      .collect::<Vec<_>>();

    for key in insert_keys {
      let value = data
        .get(key)
        .expect("should have value")
        .to_owned()
        .expect("should have value");
      res.push((key.to_owned(), value));
    }

    let remain_packs = bucket_old_pack_metas
      .iter()
      .filter(|meta| !removed_packs.contains(&meta.hash))
      .map(|meta| {
        (
          meta,
          scope_old_packs.get(&meta.hash).expect("should have pack"),
        )
      })
      .collect::<Vec<_>>();

    let mut new_packs: Vec<(PackFileMeta, Pack)> = vec![];

    for (remain_pack_meta, remain_pack) in remain_packs {
      new_scope_meta.packs[dirty_bucket_id].push(remain_pack_meta.clone());
      new_scope_packs.insert(remain_pack_meta.hash.clone(), remain_pack.clone());
    }

    for (new_pack_meta, new_pack) in new_packs {
      new_scope_meta.packs[dirty_bucket_id].push(new_pack_meta);
      new_scope_packs.insert(new_pack_meta.hash.clone(), new_pack);
    }
  }

  new_scope.packs = ScopePacksState::Value(new_scope_packs);
  new_scope.meta = ScopeMetaState::Value(new_scope_meta);

  Ok(new_scope)
}

fn get_key_bucket_id(key: &Vec<u8>, total: usize) -> usize {
  let mut hasher = FxHasher::default();
  hasher.write(key);
  let bucket_id = usize::try_from(hasher.finish() % total as u64).expect("should get bucket id");
  bucket_id
}
