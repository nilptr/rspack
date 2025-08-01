use std::marker::PhantomData;

use rspack_collections::Identifier;
use rspack_core::{Compilation, RuntimeModule, rspack_sources::Source};
use rspack_macros::impl_runtime_module;

#[allow(dead_code)]
#[test]
fn with_generic() {
  #[impl_runtime_module]
  #[derive(Debug)]
  struct Foo<T: std::fmt::Debug + Send + Sync + Eq + 'static> {
    marker: PhantomData<T>,
  }

  #[async_trait::async_trait]
  impl<T: std::fmt::Debug + Send + Sync + Eq + 'static> RuntimeModule for Foo<T> {
    fn name(&self) -> Identifier {
      todo!()
    }

    async fn generate(&self, _: &Compilation) -> rspack_error::Result<String> {
      todo!()
    }
  }
}
