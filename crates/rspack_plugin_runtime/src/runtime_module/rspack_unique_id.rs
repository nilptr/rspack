use cow_utils::CowUtils;
use rspack_collections::Identifier;
use rspack_core::{Compilation, RuntimeModule, RuntimeModuleStage, impl_runtime_module};

#[impl_runtime_module]
#[derive(Debug)]
pub struct RspackUniqueIdRuntimeModule {
  id: Identifier,
  bundler_name: String,
  bundler_version: String,
}

impl RspackUniqueIdRuntimeModule {
  pub fn new(bundler_name: String, bundler_version: String) -> Self {
    Self::with_default(
      Identifier::from("webpack/runtime/rspack_unique_id"),
      bundler_name,
      bundler_version,
    )
  }
}

#[async_trait::async_trait]
impl RuntimeModule for RspackUniqueIdRuntimeModule {
  fn stage(&self) -> RuntimeModuleStage {
    RuntimeModuleStage::Attach
  }
  fn name(&self) -> Identifier {
    self.id
  }

  async fn generate(&self, _: &Compilation) -> rspack_error::Result<String> {
    Ok(
      include_str!("runtime/get_unique_id.js")
        .cow_replace("$BUNDLER_NAME$", &self.bundler_name)
        .cow_replace("$BUNDLER_VERSION$", &self.bundler_version)
        .into_owned(),
    )
  }
}
