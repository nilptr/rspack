use rspack_error::Result;

use crate::{
  LoaderContext,
  content::{Content, ResourceData},
};

#[async_trait::async_trait]
pub trait LoaderRunnerPlugin: Send + Sync {
  type Context;

  fn name(&self) -> &'static str {
    "unknown"
  }

  async fn before_all(&self, _context: &mut LoaderContext<Self::Context>) -> Result<()> {
    Ok(())
  }

  async fn should_yield(&self, _context: &LoaderContext<Self::Context>) -> Result<bool> {
    Ok(false)
  }

  async fn start_yielding(&self, _context: &mut LoaderContext<Self::Context>) -> Result<()> {
    Ok(())
  }

  async fn process_resource(&self, resource_data: &ResourceData) -> Result<Option<Content>>;
}
