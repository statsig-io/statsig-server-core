use crate::json_utils::deserialize_as_str_map;
use napi::bindgen_prelude::ObjectFinalize;
use napi::Env;
use napi_derive::napi;
use serde_json::from_str;
use sigstat::{instance_store::INST_STORE, log_w, statsig_user::StatsigUserBuilder, DynamicValue};
use std::collections::HashMap;

const TAG: &str = "StatsigUserNapi";

#[napi(custom_finalize)]
pub struct AutoReleasingStatsigUserRef {
  pub ref_id: String,
}

impl AutoReleasingStatsigUserRef {
  fn err() -> Self {
    Self {
      ref_id: "".to_string(),
    }
  }
}
impl ObjectFinalize for AutoReleasingStatsigUserRef {
  fn finalize(self, _env: Env) -> napi::Result<()> {
    INST_STORE.remove(&self.ref_id);
    Ok(())
  }
}

#[napi]
pub fn statsig_user_create(
  user_id: Option<String>,
  custom_ids_json: Option<String>,
  email: Option<String>,
  ip: Option<String>,
  user_agent: Option<String>,
  country: Option<String>,
  locale: Option<String>,
  app_version: Option<String>,
  custom_json: Option<String>,
  private_attributes_json: Option<String>,
) -> AutoReleasingStatsigUserRef {
  // todo: extract helper functions
  let mut custom_ids = None;
  if let Some(custom_ids_json) = custom_ids_json {
    match deserialize_as_str_map(&custom_ids_json) {
      Ok(parsed_custom) => custom_ids = Some(parsed_custom),
      Err(_) => {
        log_w!(
          TAG,
          "Invalid type passed to 'CustomIDs'. Expected Record<string, string>. Received {}",
          custom_ids_json
        );
        return AutoReleasingStatsigUserRef::err();
      }
    }
  }

  let mut builder = match custom_ids {
    Some(custom_ids) => StatsigUserBuilder::new_with_custom_ids(custom_ids).user_id(user_id),
    None => {
      StatsigUserBuilder::new_with_user_id(user_id.unwrap_or_default()).custom_ids(custom_ids)
    }
  };

  let mut custom = None;
  if let Some(custom_json) = custom_json {
    match from_str::<HashMap<String, DynamicValue>>(&custom_json) {
      Ok(parsed_custom) => custom = Some(parsed_custom),
      Err(_) => {
        log_w!(TAG, "Invalid type passed to 'Custom'. Expected Record<string, string | boolean | number>. Received {}", custom_json);
        return AutoReleasingStatsigUserRef::err();
      }
    }
  }

  let mut private_attributes = None;
  if let Some(private_attributes_json) = private_attributes_json {
    match from_str::<HashMap<String, DynamicValue>>(&private_attributes_json) {
      Ok(parsed_private_attributes) => private_attributes = Some(parsed_private_attributes),
      Err(_) => {
        log_w!(TAG, "Invalid type passed to 'PrivateAttributes'. Expected Record<string, string | boolean | number>. Received {}", private_attributes_json);
        return AutoReleasingStatsigUserRef::err();
      }
    }
  }

  builder = builder
    .email(email)
    .ip(ip)
    .user_agent(user_agent)
    .country(country)
    .locale(locale)
    .app_version(app_version)
    .custom(custom)
    .private_attributes(private_attributes);

  let ref_id = INST_STORE
    .add(builder.build())
    .unwrap_or_else(|| "".to_string());

  AutoReleasingStatsigUserRef { ref_id }
}
