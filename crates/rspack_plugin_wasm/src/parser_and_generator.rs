use std::{
  borrow::Cow,
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

use indexmap::IndexMap;
use rspack_cacheable::{cacheable, cacheable_dyn, with::Unsupported};
use rspack_collections::Identifier;
use rspack_core::{
  AssetInfo, BoxDependency, BuildMetaExportsType, ChunkGraph, Compilation,
  DependencyType::WasmImport,
  ExportsInfoGetter, Filename, GenerateContext, GetUsedNameParam, Module, ModuleDependency,
  ModuleGraph, ModuleId, ModuleIdentifier, NormalModule, ParseContext, ParseResult,
  ParserAndGenerator, PathData, PrefetchExportsInfoMode, RuntimeGlobals, SourceType,
  StaticExportsDependency, StaticExportsSpec, UsedName, property_access,
  rspack_sources::{BoxSource, RawStringSource, Source, SourceExt},
};
use rspack_error::{Diagnostic, IntoTWithDiagnosticArray, Result, TWithDiagnosticArray};
use rspack_util::itoa;
use swc_core::atoms::Atom;
use wasmparser::{Import, Parser, Payload};

use crate::{ModuleIdToFileName, dependency::WasmImportDependency};

#[cacheable]
#[derive(Debug)]
pub struct AsyncWasmParserAndGenerator {
  #[cacheable(with=Unsupported)]
  pub(crate) module_id_to_filename: ModuleIdToFileName,
}

pub(crate) static WASM_SOURCE_TYPE: &[SourceType; 2] = &[SourceType::Wasm, SourceType::JavaScript];

#[cacheable_dyn]
#[async_trait::async_trait]
impl ParserAndGenerator for AsyncWasmParserAndGenerator {
  fn source_types(&self, _module: &dyn Module, _module_graph: &ModuleGraph) -> &[SourceType] {
    WASM_SOURCE_TYPE
  }

  async fn parse<'a>(
    &mut self,
    parse_context: ParseContext<'a>,
  ) -> Result<TWithDiagnosticArray<ParseResult>> {
    parse_context.build_info.strict = true;
    parse_context.build_meta.has_top_level_await = true;
    parse_context.build_meta.exports_type = BuildMetaExportsType::Namespace;

    let source = parse_context.source;

    let mut exports = Vec::with_capacity(1);
    let mut dependencies: Vec<BoxDependency> = Vec::with_capacity(1);
    let mut diagnostic = Vec::with_capacity(1);

    for payload in Parser::new(0).parse_all(&source.buffer()) {
      match payload {
        Ok(payload) => match payload {
          Payload::ExportSection(s) => {
            for export in s {
              match export {
                Ok(export) => exports.push(export.name.to_string()),
                Err(err) => diagnostic.push(Diagnostic::error(
                  "Wasm Export Parse Error".into(),
                  err.to_string(),
                )),
              };
            }
          }
          Payload::ImportSection(s) => {
            for import in s {
              match import {
                Ok(Import { module, name, ty }) => {
                  dependencies.push(Box::new(WasmImportDependency::new(
                    module.into(),
                    name.into(),
                    ty,
                  )));
                }
                Err(err) => diagnostic.push(Diagnostic::error(
                  "Wasm Import Parse Error".into(),
                  err.to_string(),
                )),
              }
            }
          }
          _ => {}
        },
        Err(err) => {
          diagnostic.push(Diagnostic::error(
            "Wasm Parse Error".into(),
            err.to_string(),
          ));
        }
      }
    }

    dependencies.push(Box::new(StaticExportsDependency::new(
      StaticExportsSpec::Array(exports.iter().cloned().map(Atom::from).collect::<Vec<_>>()),
      false,
    )));

    Ok(
      ParseResult {
        dependencies,
        blocks: vec![],
        presentational_dependencies: vec![],
        code_generation_dependencies: vec![],
        source,
        side_effects_bailout: None,
      }
      .with_diagnostic(diagnostic),
    )
  }

  fn size(&self, module: &dyn Module, source_type: Option<&SourceType>) -> f64 {
    match source_type.unwrap_or(&SourceType::Wasm) {
      SourceType::JavaScript => {
        40.0
          + module
            .get_presentational_dependencies()
            .map_or(0.0, |i| i.len() as f64 * 10.0)
      }
      SourceType::Wasm => module.source().map_or(0, |source| source.size()) as f64,
      _ => 0.0,
    }
  }

  #[allow(clippy::unwrap_in_result)]
  async fn generate(
    &self,
    source: &BoxSource,
    module: &dyn Module,
    generate_context: &mut GenerateContext,
  ) -> Result<BoxSource> {
    let GenerateContext {
      compilation,
      runtime,
      ..
    } = generate_context;
    let wasm_filename_template = &compilation.options.output.webassembly_module_filename;
    let hash = hash_for_source(source);
    let normal_module = module
      .as_normal_module()
      .expect("module should be a NormalModule in AsyncWasmParserAndGenerator::generate");
    let wasm_path_with_info =
      render_wasm_name(compilation, normal_module, wasm_filename_template, &hash).await?;

    self
      .module_id_to_filename
      .insert(module.identifier(), wasm_path_with_info.clone());

    match generate_context.requested_source_type {
      SourceType::JavaScript => {
        let runtime_requirements = &mut generate_context.runtime_requirements;
        runtime_requirements.insert(RuntimeGlobals::MODULE);
        runtime_requirements.insert(RuntimeGlobals::MODULE_ID);
        runtime_requirements.insert(RuntimeGlobals::EXPORTS);
        runtime_requirements.insert(RuntimeGlobals::INSTANTIATE_WASM);

        let mut dep_modules = IndexMap::<ModuleIdentifier, (String, &ModuleId)>::new();
        let mut wasm_deps_by_request = IndexMap::<&str, Vec<(Identifier, String, String)>>::new();
        let mut promises: Vec<String> = vec![];

        let module_graph = &compilation.get_module_graph();

        module
          .get_dependencies()
          .iter()
          .map(|id| module_graph.dependency_by_id(id).expect("should be ok"))
          .filter(|dep| dep.dependency_type() == &WasmImport)
          .map(|dep| {
            (
              dep,
              module_graph.module_graph_module_by_dependency_id(dep.id()),
            )
          })
          .for_each(|(dep, mgm)| {
            if let Some(mgm) = mgm {
              if !dep_modules.contains_key(&mgm.module_identifier) {
                let mut len_buffer = itoa::Buffer::new();
                let len_str = len_buffer.format(dep_modules.len());
                let import_var = format!("WEBPACK_IMPORTED_MODULE_{}", len_str);
                let val = (
                  import_var.clone(),
                  ChunkGraph::get_module_id(
                    &compilation.module_ids_artifact,
                    mgm.module_identifier,
                  )
                  .expect("should have module id"),
                );

                if ModuleGraph::is_async(compilation, &mgm.module_identifier) {
                  promises.push(import_var);
                }
                dep_modules.insert(mgm.module_identifier, val);
              }

              let dep = dep
                .as_any()
                .downcast_ref::<WasmImportDependency>()
                .expect("should be wasm import dependency");

              let dep_name = serde_json::to_string(dep.name()).expect("should be ok.");
              let Some(UsedName::Normal(used_name)) = ExportsInfoGetter::get_used_name(
                GetUsedNameParam::WithNames(&module_graph.get_prefetched_exports_info(
                  &mgm.module_identifier,
                  PrefetchExportsInfoMode::Default,
                )),
                *runtime,
                &[dep.name().into()],
              ) else {
                return;
              };
              let request = dep.request();
              let val = (
                mgm.module_identifier,
                dep_name,
                property_access(used_name, 0),
              );
              if let Some(deps) = wasm_deps_by_request.get_mut(&request) {
                deps.push(val);
              } else {
                wasm_deps_by_request.insert(request, vec![val]);
              }
            }
          });

        let imports_code = dep_modules
          .iter()
          .map(|(_, val)| render_import_stmt(&val.0, val.1))
          .collect::<Vec<_>>()
          .join("");

        let import_obj_request_items = wasm_deps_by_request
          .into_iter()
          .map(|(request, deps)| {
            let deps = deps
              .into_iter()
              .map(|(id, name, used_name)| {
                let import_var = dep_modules.get(&id).expect("should be ok");
                let import_var = &import_var.0;
                format!("{name}: {import_var}{used_name}")
              })
              .collect::<Vec<_>>()
              .join(",\n");

            format!(
              "{}: {{\n{deps}\n}}",
              serde_json::to_string(request).expect("should be ok")
            )
          })
          .collect::<Vec<_>>();

        let imports_obj = if !import_obj_request_items.is_empty() {
          Some(format!(
            ", {{\n{}\n}}",
            &import_obj_request_items.join(",\n")
          ))
        } else {
          None
        };

        let instantiate_call = format!(
          "{}(exports, module.id, {} {})",
          RuntimeGlobals::INSTANTIATE_WASM,
          serde_json::to_string(&hash).expect("should be ok"),
          imports_obj.unwrap_or_default()
        );

        let source = if !promises.is_empty() {
          generate_context
            .runtime_requirements
            .insert(RuntimeGlobals::ASYNC_MODULE);
          let promises = promises.join(", ");
          let decl = format!(
            "var __webpack_instantiate__ = function ([{promises}]) {{\nreturn {instantiate_call};\n}}\n",
          );
          let async_dependencies = format!(
"{}(module, async function (__webpack_handle_async_dependencies__, __webpack_async_result__) {{
  try {{
    {imports_code}
    var __webpack_async_dependencies__ = __webpack_handle_async_dependencies__([{promises}]);
    var [{promises}] = __webpack_async_dependencies__.then ? (await __webpack_async_dependencies__)() : __webpack_async_dependencies__;
    await {instantiate_call};

  __webpack_async_result__();

  }} catch(e) {{ __webpack_async_result__(e); }}
}}, 1);
",
            RuntimeGlobals::ASYNC_MODULE,
          );

          RawStringSource::from(format!("{decl}{async_dependencies}"))
        } else {
          RawStringSource::from(format!(
            "{imports_code} module.exports = {instantiate_call};"
          ))
        };

        Ok(source.boxed())
      }
      _ => Ok(source.clone()),
    }
  }

  fn get_concatenation_bailout_reason(
    &self,
    _module: &dyn Module,
    _mg: &rspack_core::ModuleGraph,
    _cg: &rspack_core::ChunkGraph,
  ) -> Option<Cow<'static, str>> {
    Some("Module Concatenation is not implemented for AsyncWasmParserAndGenerator".into())
  }
}

async fn render_wasm_name(
  compilation: &Compilation,
  normal_module: &NormalModule,
  wasm_filename_template: &Filename,
  hash: &str,
) -> Result<(String, AssetInfo)> {
  compilation
    .get_asset_path_with_info(
      wasm_filename_template,
      PathData::default()
        .module_id_optional(
          ChunkGraph::get_module_id(&compilation.module_ids_artifact, normal_module.id())
            .map(|s| PathData::prepare_id(s.as_str()))
            .as_deref(),
        )
        .filename(&normal_module.resource_resolved_data().resource)
        .content_hash(hash)
        .hash(hash),
    )
    .await
}

fn render_import_stmt(import_var: &str, module_id: &ModuleId) -> String {
  let module_id = serde_json::to_string(module_id).expect("TODO");
  format!("var {import_var} = __webpack_require__({module_id});\n",)
}

fn hash_for_source(source: &BoxSource) -> String {
  let mut hasher = DefaultHasher::new();
  source.hash(&mut hasher);
  format!("{:016x}", hasher.finish())
}
