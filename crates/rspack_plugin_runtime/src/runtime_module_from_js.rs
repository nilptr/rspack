use std::sync::Arc;

use derive_more::Debug;
use futures::future::BoxFuture;
use rspack_cacheable::with::Unsupported;
use rspack_collections::Identifier;
use rspack_core::{Compilation, RuntimeModule, RuntimeModuleStage, impl_runtime_module};

type GenerateFn = Arc<dyn Fn() -> BoxFuture<'static, rspack_error::Result<String>> + Send + Sync>;

#[impl_runtime_module]
#[derive(Debug)]
pub struct RuntimeModuleFromJs {
  pub name: String,
  #[debug(skip)]
  #[cacheable(with=Unsupported)]
  pub generator: GenerateFn,
  pub full_hash: bool,
  pub dependent_hash: bool,
  pub isolate: bool,
  pub stage: RuntimeModuleStage,
}

#[async_trait::async_trait]
impl RuntimeModule for RuntimeModuleFromJs {
  fn name(&self) -> Identifier {
    Identifier::from(format!("webpack/runtime/{}", self.name))
  }

  async fn generate(&self, _: &Compilation) -> rspack_error::Result<String> {
    let res = (self.generator)().await?;
    Ok(res)
  }

  fn full_hash(&self) -> bool {
    self.full_hash
  }

  fn dependent_hash(&self) -> bool {
    self.dependent_hash
  }

  fn should_isolate(&self) -> bool {
    self.isolate
  }

  fn stage(&self) -> RuntimeModuleStage {
    self.stage.clone()
  }
}
