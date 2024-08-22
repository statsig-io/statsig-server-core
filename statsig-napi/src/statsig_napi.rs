use std::collections::HashMap;
use napi::bindgen_prelude::ObjectFinalize;
use napi::Env;
use napi_derive::napi;
use serde_json::json;
use statsig::instance_store::{OPTIONS_INSTANCES, STATSIG_INSTANCES, USER_INSTANCES};
use statsig::{get_instance_or_else, get_instance_or_noop, get_instance_or_return, Statsig};

use crate::statsig_types_napi::{DynamicConfigNapi, ExperimentNapi, FeatureGateNapi};

#[napi(custom_finalize)]
pub struct AutoReleasingStatsigRef {
  pub value: i32,
}


impl ObjectFinalize for AutoReleasingStatsigRef {
  fn finalize(self, _env: Env) -> napi::Result<()> {
    if let Some(statsig) = STATSIG_INSTANCES.get(self.value) {
      let _ = statsig.shutdown();
      STATSIG_INSTANCES.release(self.value);
    }

    Ok(())
  }
}


#[napi]
pub fn statsig_create(sdk_key: String, options_ref: Option<i32>) -> AutoReleasingStatsigRef {
  let options = OPTIONS_INSTANCES.optional_get(options_ref);
  let statsig = Statsig::new(&sdk_key, options);

  AutoReleasingStatsigRef {
    value: STATSIG_INSTANCES.add(statsig)
  }
}

#[napi]
pub async fn statsig_initialize(statsig_ref: i32) {
  let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);
  let _ = statsig.initialize().await;
}

#[napi]
pub async fn statsig_shutdown(statsig_ref: i32) {
  let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);
  let _ = statsig.shutdown().await;
}

#[napi]
pub fn statsig_get_current_values(statsig_ref: i32) -> Option<String> {
  let statsig = get_instance_or_return!(STATSIG_INSTANCES, statsig_ref, None);

  match statsig.get_current_values() {
    Some(d) => Some(json!(d).to_string()),
    _ => None,
  }
}

#[napi]
pub fn statsig_log_string_value_event(
  statsig_ref: i32,
  user_ref: i32,
  event_name: String,
  value: Option<String>,
  metadata: Option<HashMap<String, String>>,
) {
  let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);
  let user = get_instance_or_noop!(USER_INSTANCES, user_ref);

  statsig.log_event(&user, &event_name, value, metadata);
}

#[napi]
pub fn statsig_check_gate(statsig_ref: i32, user_ref: i32, gate_name: String) -> bool {
  let statsig = get_instance_or_return!(STATSIG_INSTANCES, statsig_ref, false);
  let user = get_instance_or_return!(USER_INSTANCES, user_ref, false);

  statsig.check_gate(&user, &gate_name)
}

#[napi]
pub fn statsig_get_feature_gate(
  statsig_ref: i32,
  user_ref: i32,
  gate_name: String,
) -> FeatureGateNapi {
  let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
    return create_empty_feature_gate(gate_name);
  });

  let user = get_instance_or_else!(USER_INSTANCES, user_ref, {
    return create_empty_feature_gate(gate_name);
  });

  let gate = statsig.get_feature_gate(&user, &gate_name);

  FeatureGateNapi {
    name: gate_name,
    rule_id: gate.rule_id,
    id_type: gate.id_type,
    value: gate.value,
  }
}

#[napi]
pub fn statsig_get_dynamic_config(
  statsig_ref: i32,
  user_ref: i32,
  dynamic_config_name: String,
) -> DynamicConfigNapi {
  let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
    return create_empty_dynamic_config(dynamic_config_name);
  });

  let user = get_instance_or_else!(USER_INSTANCES, user_ref, {
    return create_empty_dynamic_config(dynamic_config_name);
  });

  let dynamic_config = statsig.get_dynamic_config(&user, &dynamic_config_name);

  DynamicConfigNapi {
    name: dynamic_config_name,
    rule_id: dynamic_config.rule_id,
    id_type: dynamic_config.id_type,
    json_value: json!(dynamic_config.value).to_string(),
  }
}

#[napi]
pub fn statsig_get_experiment(
  statsig_ref: i32,
  user_ref: i32,
  experiment_name: String,
) -> ExperimentNapi {
  let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
    return create_empty_experiment(experiment_name);
  });

  let user = get_instance_or_else!(USER_INSTANCES, user_ref, {
    return create_empty_experiment(experiment_name);
  });

  let experiment = statsig.get_experiment(&user, &experiment_name);

  ExperimentNapi {
    name: experiment_name,
    rule_id: experiment.rule_id,
    id_type: experiment.id_type,
    group_name: experiment.group_name,
    json_value: json!(experiment.value).to_string(),
  }
}

#[napi]
pub fn statsig_get_layer(statsig_ref: i32, user_ref: i32, layer_name: String) -> String {
  let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
    return create_empty_layer_json(layer_name);
  });

  let user = get_instance_or_else!(USER_INSTANCES, user_ref, {
    return create_empty_layer_json(layer_name);
  });

  let layer = statsig.get_layer(&user, &layer_name);
  json!(layer).to_string()
}

#[napi]
pub fn statsig_log_layer_param_exposure(statsig_ref: i32, layer_data: String, param_name: String) {
  let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);

  statsig.log_layer_param_exposure(layer_data, param_name)
}

#[napi]
pub fn statsig_get_client_init_response(statsig_ref: i32, user_ref: i32) -> String {
  let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
    return String::from("{}");
  });

  let user = get_instance_or_else!(USER_INSTANCES, user_ref, {
    return String::from("{}");
  });

  let response = statsig.get_client_init_response(&user);
  json!(response).to_string()
}

// -------
// Private
// -------

fn create_empty_feature_gate(name: String) -> FeatureGateNapi {
  FeatureGateNapi {
    name,
    rule_id: String::new(),
    id_type: String::new(),
    value: false,
  }
}

fn create_empty_dynamic_config(name: String) -> DynamicConfigNapi {
  DynamicConfigNapi {
    name,
    rule_id: String::new(),
    id_type: String::new(),
    json_value: String::from("{}"),
  }
}

fn create_empty_experiment(name: String) -> ExperimentNapi {
  ExperimentNapi {
    name,
    rule_id: String::new(),
    id_type: String::new(),
    group_name: None,
    json_value: String::from("{}"),
  }
}

fn create_empty_layer_json(name: String) -> String {
  format!("\"name\": \"{}\"", name)
}
