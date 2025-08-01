use rspack_collections::Identifier;
use rspack_core::{Compilation, RuntimeModule, impl_runtime_module};

#[impl_runtime_module]
#[derive(Debug)]
pub struct RspackVersionRuntimeModule {
  id: Identifier,
  version: String,
}

impl RspackVersionRuntimeModule {
  pub fn new(version: String) -> Self {
    Self::with_default(Identifier::from("webpack/runtime/rspack_version"), version)
  }
}

#[async_trait::async_trait]
impl RuntimeModule for RspackVersionRuntimeModule {
  fn name(&self) -> Identifier {
    self.id
  }

  fn template(&self) -> Vec<(String, String)> {
    vec![(
      self.id.to_string(),
      include_str!("runtime/get_version.ejs").to_string(),
    )]
  }

  async fn generate(&self, compilation: &Compilation) -> rspack_error::Result<String> {
    let source = compilation.runtime_template.render(
      &self.id,
      Some(serde_json::json!({
        "_version": format!("\"{}\"", &self.version),
      })),
    )?;

    Ok(source)
  }
}
