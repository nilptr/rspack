use std::{any::Any, hash::BuildHasherDefault, sync::Arc};

use dashmap::DashMap;
use rspack_cacheable::{
  DeserializeError, SerializeError, cacheable,
  with::{As, AsConverter},
};
use rspack_fs::ReadableFileSystem;
use rustc_hash::FxHasher;

use super::resolver_impl::Resolver;
use crate::{DependencyCategory, Resolve, cache::persistent::CacheableContext};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
// Actually this should be ResolveOptionsWithDependencyCategory, it's a mistake from webpack, but keep the alignment for easily find the code in webpack
pub struct ResolveOptionsWithDependencyType {
  pub resolve_options: Option<Box<Resolve>>,
  pub resolve_to_context: bool,
  pub dependency_category: DependencyCategory,
}

#[cacheable(with=As::<Resolve>)]
#[derive(Debug)]
pub struct ResolverFactory {
  base_options: Resolve,
  resolver: Resolver,
  /// Different resolvers are used for different resolution strategies such as ESM and CJS.
  /// All resolvers share the same underlying cache.
  resolvers: DashMap<ResolveOptionsWithDependencyType, Arc<Resolver>, BuildHasherDefault<FxHasher>>,
}

impl AsConverter<ResolverFactory> for Resolve {
  fn serialize(data: &ResolverFactory, _ctx: &dyn Any) -> Result<Self, SerializeError> {
    Ok(data.base_options.clone())
  }
  fn deserialize(self, ctx: &dyn Any) -> Result<ResolverFactory, DeserializeError> {
    let Some(ctx) = ctx.downcast_ref::<CacheableContext>() else {
      return Err(DeserializeError::NoContext);
    };
    Ok(ResolverFactory::new(self, ctx.input_filesystem.clone()))
  }
}

impl ResolverFactory {
  pub fn clear_cache(&self) {
    self.resolver.clear_cache();
  }

  pub fn new(options: Resolve, fs: Arc<dyn ReadableFileSystem>) -> Self {
    Self {
      base_options: options.clone(),
      resolver: Resolver::new(options, fs),
      resolvers: Default::default(),
    }
  }

  pub fn get(&self, options: ResolveOptionsWithDependencyType) -> Arc<Resolver> {
    if let Some(r) = self.resolvers.get(&options) {
      r.clone()
    } else {
      let base_options = self.base_options.clone();
      let merged_options = match &options.resolve_options {
        Some(o) => base_options.merge(*o.clone()),
        None => base_options,
      };
      let resolver = Arc::new(self.resolver.clone_with_options(merged_options, &options));
      self.resolvers.insert(options, resolver.clone());
      resolver
    }
  }
}
