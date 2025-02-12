use napi_derive::napi;
use sigstat::{EvaluationDetails, SecondaryExposure};

#[napi(object, js_name = "EvaluationDetails")]
pub struct EvaluationDetailsNapi {
  pub reason: String,
  pub lcut: Option<i64>,
  pub received_at: Option<i64>,
}

impl From<EvaluationDetails> for EvaluationDetailsNapi {
  fn from(value: EvaluationDetails) -> Self {
    EvaluationDetailsNapi {
      reason: value.reason,
      lcut: value.lcut.map(|lcut| lcut as i64),
      received_at: value.received_at.map(|t| t as i64),
    }
  }
}

#[napi(object, js_name = "SecondaryExposure")]
pub struct SecondaryExposureNapi {
  pub gate: String,
  pub gate_value: String,
  pub rule_id: String,
}

impl From<SecondaryExposure> for SecondaryExposureNapi {
  fn from(value: SecondaryExposure) -> Self {
    SecondaryExposureNapi {
      gate: value.gate,
      gate_value: value.gate_value,
      rule_id: value.rule_id,
    }
  }
}

#[napi(object)]
pub struct FeatureGateNapi {
  pub name: String,
  pub value: bool,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,
  pub evaluation_details: Option<EvaluationDetailsNapi>,
}

#[napi(object)]
pub struct DynamicConfigNapi {
  pub name: String,
  pub json_value: String,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,
  pub secondary_exposures: Option<Vec<SecondaryExposureNapi>>,
  pub evaluation_details: Option<EvaluationDetailsNapi>,
}

#[napi(object)]
pub struct ExperimentNapi {
  pub name: String,
  pub json_value: String,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,
  pub group_name: Option<String>,
  pub secondary_exposures: Option<Vec<SecondaryExposureNapi>>,
  pub evaluation_details: Option<EvaluationDetailsNapi>,
}

#[napi(object)]
pub struct LayerNapi {
  pub name: String,
  #[napi(js_name = "ruleID")]
  pub rule_id: String,
  pub id_type: String,
  pub group_name: Option<String>,
  pub allocated_experiment_name: Option<String>,

  #[napi(js_name = "__jsonValue")]
  pub __json_value: String,
  pub __json_user: String,
  pub evaluation_details: Option<EvaluationDetailsNapi>,
}
