use napi_derive::napi;
use statsig::{instance_store::USER_INSTANCES, statsig_user::StatsigUserBuilder};
use std::collections::HashMap;

#[napi]
pub fn statsig_user_create(
  user_id: Option<String>,
  custom_ids: Option<HashMap<String, String>>,
  email: Option<String>,
  ip: Option<String>,
  user_agent: Option<String>,
  country: Option<String>,
  locale: Option<String>,
  app_version: Option<String>,
  custom: Option<HashMap<String, String>>,
  private_attributes: Option<HashMap<String, String>>,
) -> i32 {
  let mut builder = match custom_ids {
    Some(custom_ids) => StatsigUserBuilder::new_with_custom_ids(custom_ids).user_id(user_id),
    None => {
      StatsigUserBuilder::new_with_user_id(user_id.unwrap_or_default()).custom_ids(custom_ids)
    }
  };

  builder = builder
    .email(email)
    .ip(ip)
    .user_agent(user_agent)
    .country(country)
    .locale(locale)
    .app_version(app_version)
    .custom(custom)
    .private_attributes(private_attributes);

  USER_INSTANCES.add(builder.build())
}

#[napi]
pub fn statsig_user_release(user_ref: i32) {
  USER_INSTANCES.release(user_ref)
}
