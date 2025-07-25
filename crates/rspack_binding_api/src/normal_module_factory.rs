use napi_derive::napi;
use rspack_core::NormalModuleCreateData;
use serde::Serialize;

use crate::JsResourceData;

#[napi(object)]
pub struct JsResolveForSchemeArgs {
  pub resource_data: JsResourceData,
  pub scheme: String,
}

pub type JsResolveForSchemeOutput = (Option<bool>, JsResourceData);

#[napi(object)]
pub struct JsBeforeResolveArgs {
  pub request: String,
  pub context: String,
  pub issuer: String,
  pub issuer_layer: Option<String>,
}

pub type JsBeforeResolveOutput = (Option<bool>, JsBeforeResolveArgs);

#[napi(object)]
pub struct JsFactorizeArgs {
  pub request: String,
  pub context: String,
  pub issuer: String,
  pub issuer_layer: Option<String>,
}

pub type JsFactorizeOutput = JsFactorizeArgs;

#[napi(object)]
pub struct JsResolveArgs {
  pub request: String,
  pub context: String,
  pub issuer: String,
  pub issuer_layer: Option<String>,
}

pub type JsResolveOutput = JsResolveArgs;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct JsCreateData {
  pub request: String,
  pub user_request: String,
  pub resource: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsAfterResolveData {
  pub request: String,
  pub context: String,
  pub issuer: String,
  pub issuer_layer: Option<String>,
  pub file_dependencies: Vec<String>,
  pub context_dependencies: Vec<String>,
  pub missing_dependencies: Vec<String>,
  pub create_data: Option<JsCreateData>,
}

pub type JsAfterResolveOutput = (Option<bool>, Option<JsCreateData>);

#[napi(object)]
pub struct JsNormalModuleFactoryCreateModuleArgs {
  pub dependency_type: String,
  pub raw_request: String,
  pub resource_resolve_data: JsResourceData,
  pub context: String,
  pub match_resource: Option<String>,
}

impl From<&NormalModuleCreateData> for JsCreateData {
  fn from(value: &NormalModuleCreateData) -> Self {
    Self {
      request: value.request.to_owned(),
      user_request: value.user_request.to_owned(),
      resource: value.resource_resolve_data.resource.to_owned(),
    }
  }
}
