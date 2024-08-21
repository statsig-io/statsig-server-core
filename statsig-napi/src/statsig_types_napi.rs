use napi_derive::napi;

#[napi(object)]
pub struct FeatureGateNapi {
  pub name: String,
  pub value: bool,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,
}

#[napi(object)]
pub struct DynamicConfigNapi {
  pub name: String,
  pub json_value: String,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,
}

#[napi(object)]
pub struct ExperimentNapi {
  pub name: String,
  pub json_value: String,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,
}

#[napi(object)]
pub struct LayerNapi {
  pub name: String,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,

  #[napi(js_name = "__jsonValue")]
  pub __json_value: String,
}
