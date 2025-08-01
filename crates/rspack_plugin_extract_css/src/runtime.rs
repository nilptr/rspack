use cow_utils::CowUtils;
use rspack_collections::UkeySet;
use rspack_core::{
  ChunkUkey, Compilation, CrossOriginLoading, RuntimeGlobals, RuntimeModule, RuntimeModuleStage,
  impl_runtime_module,
};
use rspack_error::Result;
use rspack_plugin_runtime::get_chunk_runtime_requirements;
use rustc_hash::FxHashMap;

use crate::plugin::{InsertType, SOURCE_TYPE};

static RUNTIME_CODE: &str = include_str!("./runtime/css_load.js");
static WITH_LOADING: &str = include_str!("./runtime/with_loading.js");
static WITH_HMR: &str = include_str!("./runtime/with_hmr.js");

#[impl_runtime_module]
#[derive(Debug)]
pub(crate) struct CssLoadingRuntimeModule {
  chunk: ChunkUkey,
  attributes: FxHashMap<String, String>,
  link_type: Option<String>,
  insert: InsertType,
}

impl CssLoadingRuntimeModule {
  pub(crate) fn new(
    chunk: ChunkUkey,
    attributes: FxHashMap<String, String>,
    link_type: Option<String>,
    insert: InsertType,
  ) -> Self {
    Self::with_default(chunk, attributes, link_type, insert)
  }

  fn get_css_chunks(&self, compilation: &Compilation) -> UkeySet<ChunkUkey> {
    let mut set: UkeySet<ChunkUkey> = Default::default();
    let module_graph = compilation.get_module_graph();

    let chunk = compilation.chunk_by_ukey.expect_get(&self.chunk);

    for chunk in chunk.get_all_async_chunks(&compilation.chunk_group_by_ukey) {
      let modules = compilation
        .chunk_graph
        .get_chunk_modules_iterable_by_source_type(&chunk, SOURCE_TYPE[0], &module_graph);

      if modules.count() > 0 {
        set.insert(chunk);
      }
    }

    set
  }
}

#[async_trait::async_trait]
impl RuntimeModule for CssLoadingRuntimeModule {
  fn name(&self) -> rspack_collections::Identifier {
    "webpack/runtime/css loading".into()
  }

  fn stage(&self) -> RuntimeModuleStage {
    RuntimeModuleStage::Attach
  }

  async fn generate(&self, compilation: &rspack_core::Compilation) -> Result<String> {
    let runtime = RUNTIME_CODE;
    let runtime_requirements = get_chunk_runtime_requirements(compilation, &self.chunk);

    let with_loading = runtime_requirements.contains(RuntimeGlobals::ENSURE_CHUNK_HANDLERS) && {
      let chunk = compilation.chunk_by_ukey.expect_get(&self.chunk);

      chunk
        .get_all_async_chunks(&compilation.chunk_group_by_ukey)
        .iter()
        .any(|chunk| {
          !compilation
            .chunk_graph
            .get_chunk_modules_by_source_type(
              chunk,
              SOURCE_TYPE[0],
              &compilation.get_module_graph(),
            )
            .is_empty()
        })
    };

    let with_hmr = runtime_requirements.contains(RuntimeGlobals::HMR_DOWNLOAD_UPDATE_HANDLERS);

    if !with_hmr && !with_loading {
      return Ok("".to_string());
    }

    let mut attr = String::default();
    let mut attributes: Vec<(&String, &String)> = self.attributes.iter().collect::<Vec<_>>();
    attributes.sort_unstable_by(|(k1, _), (k2, _)| k1.cmp(k2));

    for (attr_key, attr_value) in attributes {
      attr += &format!("linkTag.setAttribute({attr_key}, {attr_value});\n");
    }
    let runtime = runtime.cow_replace("__SET_ATTRIBUTES__", &attr);

    let runtime = if let Some(link_type) = &self.link_type {
      runtime.cow_replace("__SET_LINKTYPE__", &format!("linkTag.type={link_type};"))
    } else {
      runtime.cow_replace("__SET_LINKTYPE__", "")
    };

    let runtime = if let CrossOriginLoading::Enable(cross_origin_loading) =
      &compilation.options.output.cross_origin_loading
    {
      runtime.cow_replace(
        "__CROSS_ORIGIN_LOADING__",
        &format!(
          "if (linkTag.href.indexOf(window.location.origin + '/') !== 0) {{
  linkTag.crossOrigin = \"{cross_origin_loading}\";
}}"
        ),
      )
    } else {
      runtime.cow_replace("__CROSS_ORIGIN_LOADING__", "")
    };

    let runtime = match &self.insert {
      InsertType::Fn(f) => runtime.cow_replace("__INSERT__", &format!("({f})(linkTag);")),
      InsertType::Selector(sel) => runtime.cow_replace(
        "__INSERT__",
        &format!("var target = document.querySelector({sel});\ntarget.parentNode.insertBefore(linkTag, target.nextSibling);"),
      ),
      InsertType::Default => runtime.cow_replace(
        "__INSERT__",
        "if (oldTag) {
  oldTag.parentNode.insertBefore(linkTag, oldTag.nextSibling);
} else {
  document.head.appendChild(linkTag);
}",
      ),
    };

    let runtime = if with_loading {
      let chunks = self.get_css_chunks(compilation);
      if chunks.is_empty() {
        runtime.cow_replace("__WITH_LOADING__", "// no chunk loading")
      } else {
        let chunk = compilation.chunk_by_ukey.expect_get(&self.chunk);
        let with_loading = WITH_LOADING.cow_replace(
          "__INSTALLED_CHUNKS__",
          &format!(
            "{}: 0,\n",
            serde_json::to_string(chunk.expect_id(&compilation.chunk_ids_artifact))
              .expect("json stringify failed")
          ),
        );

        let with_loading = with_loading.cow_replace(
          "__ENSURE_CHUNK_HANDLERS__",
          &RuntimeGlobals::ENSURE_CHUNK_HANDLERS.to_string(),
        );

        let with_loading = with_loading.cow_replace(
          "__CSS_CHUNKS__",
          &format!(
            "{{\n{}\n}}",
            chunks
              .iter()
              .filter_map(|id| {
                let chunk = compilation.chunk_by_ukey.expect_get(id);

                chunk.id(&compilation.chunk_ids_artifact).map(|id| {
                  format!(
                    "{}: 1,\n",
                    serde_json::to_string(id).expect("json stringify failed")
                  )
                })
              })
              .collect::<String>()
          ),
        );

        runtime.cow_replace("__WITH_LOADING__", &with_loading)
      }
    } else {
      runtime.cow_replace("__WITH_LOADING__", "// no chunk loading")
    };

    let runtime = if with_hmr {
      runtime.cow_replace(
        "__WITH_HMT__",
        &WITH_HMR.cow_replace(
          "__HMR_DOWNLOAD__",
          &RuntimeGlobals::HMR_DOWNLOAD_UPDATE_HANDLERS.to_string(),
        ),
      )
    } else {
      runtime.cow_replace("__WITH_HMT__", "// no hmr")
    };

    Ok(runtime.into_owned())
  }
}
