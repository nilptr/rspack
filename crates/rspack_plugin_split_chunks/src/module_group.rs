use derive_more::Debug;
use rspack_collections::{IdentifierSet, UkeySet};
use rspack_core::{ChunkUkey, ModuleIdentifier, SourceType};
use rustc_hash::FxHashMap;

use crate::{
  CacheGroup,
  common::{ModuleSizes, SplitChunkSizes},
};

/// `ModuleGroup` is a abstraction of middle step for splitting chunks.
///
/// `ModuleGroup` captures/contains a bunch of modules due to the `optimization.splitChunks` configuration.
///
/// `ModuleGroup` would be transform into `Chunk`s in the end.
///
///  A `ModuleGroup` would be transform into multiple `Chunk`s if the `name` dynamic computed
///
/// The original name of `ModuleGroup` is `ChunkInfoItem` borrowed from Webpack
#[derive(Debug)]
pub(crate) struct ModuleGroup {
  /// the real index used for mapping the ModuleGroup to corresponding CacheGroup
  idx: CacheGroupIdx,
  pub modules: IdentifierSet,
  pub cache_group_index: usize,
  pub cache_group_priority: f64,
  pub cache_group_reuse_existing_chunk: bool,
  /// If the `ModuleGroup` is going to create a chunk, which will be named using `chunk_name`
  /// A module
  pub chunk_name: Option<String>,
  pub sizes: SplitChunkSizes,
  pub source_types_modules: FxHashMap<SourceType, IdentifierSet>,
  /// `Chunk`s which `Module`s in this ModuleGroup belong to
  #[debug(skip)]
  pub chunks: UkeySet<ChunkUkey>,
}

impl ModuleGroup {
  pub fn new(
    idx: CacheGroupIdx,
    chunk_name: Option<String>,
    cache_group_index: usize,
    cache_group: &CacheGroup,
  ) -> Self {
    Self {
      idx,
      modules: Default::default(),
      cache_group_index,
      cache_group_priority: cache_group.priority,
      cache_group_reuse_existing_chunk: cache_group.reuse_existing_chunk,
      sizes: Default::default(),
      source_types_modules: Default::default(),
      chunks: Default::default(),
      chunk_name,
    }
  }

  pub fn get_source_types_modules(
    &self,
    ty: &[SourceType],
    module_sizes: &ModuleSizes,
  ) -> IdentifierSet {
    // if there is only one source type, we can just use the `source_types_modules` directly
    // instead of iterating over all modules
    if ty.len() == 1 {
      self
        .source_types_modules
        .get(ty.first().expect("should have at least one source type"))
        .cloned()
        .unwrap_or_default()
    } else {
      self
        .modules
        .iter()
        .filter_map(|module| {
          let sizes = module_sizes.get(module).expect("should have module size");
          if ty.iter().any(|ty| sizes.contains_key(ty)) {
            Some(*module)
          } else {
            None
          }
        })
        .collect()
    }
  }

  pub fn add_module(&mut self, module: ModuleIdentifier, module_sizes: &ModuleSizes) {
    if self.modules.insert(module) {
      let module_sizes = module_sizes.get(&module).expect("should have module size");
      for (ty, s) in module_sizes.iter() {
        let size = self.sizes.entry(*ty).or_default();
        *size += s;
        self
          .source_types_modules
          .entry(*ty)
          .or_default()
          .insert(module);
      }
    }
  }

  pub fn remove_module(&mut self, module: ModuleIdentifier, module_sizes: &ModuleSizes) {
    if self.modules.remove(&module) {
      let module_sizes = module_sizes.get(&module).expect("should have module size");
      for (ty, s) in module_sizes.iter() {
        let size = self.sizes.entry(*ty).or_default();
        *size -= s;
        *size = size.max(0.0);
        self
          .source_types_modules
          .entry(*ty)
          .or_default()
          .remove(&module);
      }
    }
  }

  pub fn get_cache_group<'a>(&self, cache_groups: &'a [CacheGroup]) -> &'a CacheGroup {
    &cache_groups[self.idx.0]
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CacheGroupIdx(usize);

impl CacheGroupIdx {
  pub fn new(idx: usize) -> Self {
    Self(idx)
  }
}

pub(crate) fn compare_entries(a: &ModuleGroup, b: &ModuleGroup) -> f64 {
  // 1. by priority
  let diff_priority = a.cache_group_priority - b.cache_group_priority;
  if diff_priority != 0f64 {
    return diff_priority;
  }
  // 2. by number of chunks
  let diff_count = a.chunks.len() as f64 - b.chunks.len() as f64;
  if diff_count != 0f64 {
    return diff_count;
  }

  // 3. by size reduction
  let a_size_reduce = total_size(&a.sizes) * (a.chunks.len() - 1) as f64;
  let b_size_reduce = total_size(&b.sizes) * (b.chunks.len() - 1) as f64;
  let diff_size_reduce = a_size_reduce - b_size_reduce;
  if diff_size_reduce != 0f64 {
    return diff_size_reduce;
  }

  // 4. by cache group index
  let index_diff = b.cache_group_index as f64 - a.cache_group_index as f64;
  if index_diff != 0f64 {
    return index_diff;
  }

  // 5. by number of modules (to be able to compare by identifier)
  let modules_a_len = a.modules.len();
  let modules_b_len = b.modules.len();
  let diff = modules_a_len as f64 - modules_b_len as f64;
  if diff != 0f64 {
    return diff;
  }

  let mut modules_a = a.modules.iter().collect::<Vec<_>>();
  let mut modules_b = b.modules.iter().collect::<Vec<_>>();
  modules_a.sort_unstable();
  modules_b.sort_unstable();

  loop {
    match (modules_a.pop(), modules_b.pop()) {
      (None, None) => return 0f64,
      (Some(a), Some(b)) => {
        let res = a.cmp(b);
        if !res.is_eq() {
          return res as i32 as f64;
        }
      }
      _ => unreachable!(),
    }
  }
}

fn total_size(sizes: &SplitChunkSizes) -> f64 {
  let mut size = 0f64;
  for ty_size in sizes.0.values() {
    size += ty_size;
  }
  size
}
