use napi::bindgen_prelude::Promise;
use napi_derive::napi;
use rspack_napi::threadsafe_function::ThreadsafeFunction;
use rspack_plugin_dynamic_entry::{DynamicEntryPluginOptions, EntryDynamicResult};

use crate::entry::JsEntryOptions;

#[derive(Debug)]
#[napi(object, object_to_js = false)]
pub struct RawEntryDynamicResult {
  pub import: Vec<String>,
  pub options: JsEntryOptions,
}

pub type RawEntryDynamic = ThreadsafeFunction<(), Promise<Vec<RawEntryDynamicResult>>>;

#[derive(Debug)]
#[napi(object, object_to_js = false)]
pub struct RawDynamicEntryPluginOptions {
  pub context: String,
  #[napi(ts_type = "() => Promise<RawEntryDynamicResult[]>")]
  pub entry: RawEntryDynamic,
}

impl From<RawDynamicEntryPluginOptions> for DynamicEntryPluginOptions {
  fn from(opts: RawDynamicEntryPluginOptions) -> Self {
    Self {
      context: opts.context.into(),
      entry: Box::new(move || {
        let f = opts.entry.clone();
        Box::pin(async move {
          let raw_result = f.call_with_promise(()).await?;
          let result = raw_result
            .into_iter()
            .map(
              |RawEntryDynamicResult { import, options }| EntryDynamicResult {
                import,
                options: options.into(),
              },
            )
            .collect::<Vec<_>>();
          Ok(result)
        })
      }),
    }
  }
}
