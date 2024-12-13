use napi::bindgen_prelude::ObjectFinalize;
use napi::Env;
use napi_derive::napi;
use serde_json::json;
use sigstat::instance_store::INST_STORE;
use sigstat::{
  get_instance_or_else, get_instance_or_noop, get_instance_or_return, log_e,
  ClientInitResponseOptions, HashAlgorithm, Statsig, StatsigOptions, StatsigUser,
};
use std::collections::HashMap;
use std::time::Duration;

use crate::statsig_types_napi::{DynamicConfigNapi, ExperimentNapi, FeatureGateNapi};

const TAG: &str = "StatsigNapi";

#[napi(custom_finalize)]
pub struct AutoReleasingStatsigRef {
  pub ref_id: String,
}

impl ObjectFinalize for AutoReleasingStatsigRef {
  fn finalize(self, _env: Env) -> napi::Result<()> {
    if let Some(statsig) = INST_STORE.get::<Statsig>(&self.ref_id) {
      let inst = statsig.clone();
      let rt = statsig.statsig_runtime.clone();
      rt.runtime_handle.spawn(async move {
        if let Err(e) = inst.__shutdown_internal(Duration::from_secs(3)).await {
          log_e!(TAG, "Failed to gracefully shutdown StatsigNapi: {}", e);
        }
      });

      INST_STORE.remove(&self.ref_id);
    }

    Ok(())
  }
}

#[napi]
pub fn statsig_create(sdk_key: String, options_ref: Option<String>) -> AutoReleasingStatsigRef {
  let options = INST_STORE.get_with_optional_id::<StatsigOptions>(options_ref.as_ref());
  let statsig = Statsig::new(&sdk_key, options);

  let ref_id = INST_STORE.add(statsig).unwrap_or_else(|| {
    log_e!(TAG, "Failed to create Statsig instance");
    "".to_string()
  });

  AutoReleasingStatsigRef { ref_id }
}

#[napi]
pub async fn statsig_initialize(statsig_ref: String) {
  let statsig = get_instance_or_noop!(Statsig, &statsig_ref);
  let _ = statsig.initialize().await;
}

#[napi]
pub async fn statsig_shutdown(statsig_ref: String) {
  let statsig = get_instance_or_noop!(Statsig, &statsig_ref);
  let _ = statsig.shutdown().await;
}

#[napi]
pub fn statsig_get_current_values(statsig_ref: String) -> Option<String> {
  let statsig = get_instance_or_return!(Statsig, &statsig_ref, None);

  statsig.get_current_values().map(|d| json!(d).to_string())
}

#[napi]
pub fn statsig_log_string_value_event(
  statsig_ref: String,
  user_ref: String,
  event_name: String,
  value: Option<String>,
  metadata: Option<HashMap<String, String>>,
) {
  let statsig = get_instance_or_noop!(Statsig, &statsig_ref);
  let user = get_instance_or_noop!(StatsigUser, &user_ref);

  statsig.log_event(&user, &event_name, value, metadata);
}

#[napi]
pub fn statsig_check_gate(statsig_ref: String, user_ref: String, gate_name: String) -> bool {
  let statsig = get_instance_or_return!(Statsig, &statsig_ref, false);
  let user = get_instance_or_return!(StatsigUser, &user_ref, false);

  statsig.check_gate(&user, &gate_name)
}

#[napi]
pub fn statsig_get_feature_gate(
  statsig_ref: String,
  user_ref: String,
  gate_name: String,
) -> FeatureGateNapi {
  let statsig = get_instance_or_else!(Statsig, &statsig_ref, {
    return create_empty_feature_gate(gate_name);
  });

  let user = get_instance_or_else!(StatsigUser, &user_ref, {
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
  statsig_ref: String,
  user_ref: String,
  dynamic_config_name: String,
) -> DynamicConfigNapi {
  let statsig = get_instance_or_else!(Statsig, &statsig_ref, {
    return create_empty_dynamic_config(dynamic_config_name);
  });

  let user = get_instance_or_else!(StatsigUser, &user_ref, {
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
  statsig_ref: String,
  user_ref: String,
  experiment_name: String,
) -> ExperimentNapi {
  let statsig = get_instance_or_else!(Statsig, &statsig_ref, {
    return create_empty_experiment(experiment_name);
  });

  let user = get_instance_or_else!(StatsigUser, &user_ref, {
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
pub fn statsig_get_layer(statsig_ref: String, user_ref: String, layer_name: String) -> String {
  let statsig = get_instance_or_else!(Statsig, &statsig_ref, {
    return create_empty_layer_json(layer_name);
  });

  let user = get_instance_or_else!(StatsigUser, &user_ref, {
    return create_empty_layer_json(layer_name);
  });

  let layer = statsig.get_layer(&user, &layer_name);
  json!(layer).to_string()
}

#[napi]
pub fn statsig_log_layer_param_exposure(
  statsig_ref: String,
  layer_data: String,
  param_name: String,
) {
  let statsig = get_instance_or_noop!(Statsig, &statsig_ref);

  statsig.log_layer_param_exposure(layer_data, param_name)
}

#[napi(object, js_name = "ClientInitResponseOptions")]
pub struct ClientInitResponseOptionsNapi {
  pub hash_algorithm: Option<String>,
}

#[napi]
pub fn statsig_get_client_init_response(
  statsig_ref: String,
  user_ref: String,
  options: Option<ClientInitResponseOptionsNapi>,
) -> String {
  let statsig = get_instance_or_else!(Statsig, &statsig_ref, {
    return String::from("{}");
  });

  let user = get_instance_or_else!(StatsigUser, &user_ref, {
    return String::from("{}");
  });

  let options = convert_client_init_response_options(&options);

  let response = match options.as_ref() {
    Some(options) => statsig.get_client_init_response_with_options(&user, options),
    None => statsig.get_client_init_response(&user),
  };

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

fn convert_client_init_response_options(
  options: &Option<ClientInitResponseOptionsNapi>,
) -> Option<ClientInitResponseOptions> {
  let options = match options {
    Some(options) => options,
    None => return None,
  };

  let hash_algorithm = options
    .hash_algorithm
    .as_ref()
    .and_then(|s| HashAlgorithm::from_string(s.as_str()));

  Some(ClientInitResponseOptions {
    hash_algorithm,
    ..ClientInitResponseOptions::default()
  })
}
