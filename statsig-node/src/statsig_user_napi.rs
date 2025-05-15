use std::{collections::HashMap, sync::Arc};

use napi::bindgen_prelude::{Either3, Either4};
use napi_derive::napi;
use serde_json::Value;
use statsig_rust::{
    dyn_value, log_w, user::user_data::UserData, DynamicValue, StatsigUser as StatsigUserActual,
};

const TAG: &str = "StatsigUserNapi";

type ValidPrimitives = Either4<String, f64, bool, Vec<Value>>;

#[napi(object)]
pub struct StatsigUserArgs {
    #[napi(js_name = "userID")]
    pub user_id: Option<String>,
    #[napi(js_name = "customIDs", ts_type = "Record<string, string>")]
    pub custom_ids: Option<HashMap<String, Either3<String, f64, i64>>>,
    pub email: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub country: Option<String>,
    pub locale: Option<String>,
    pub app_version: Option<String>,

    #[napi(
        ts_type = "Record<string, string | number | boolean | Array<string | number | boolean>>"
    )]
    pub custom: Option<HashMap<String, ValidPrimitives>>,

    #[napi(
        ts_type = "Record<string, string | number | boolean | Array<string | number | boolean>>"
    )]
    pub private_attributes: Option<HashMap<String, ValidPrimitives>>,
}

#[napi]
pub struct StatsigUser {
    inner: StatsigUserActual,
}

macro_rules! set_dynamic_value_fields {
    ($args:ident, $inner:ident, $($field:ident),*) => {
        $(
            if let Some(value) = $args.$field {
                $inner.$field = Some(dyn_value!(value));
            }
        )*
    };
}

fn unidentifiable_user() -> UserData {
    log_w!(TAG, "Must pass a valid user with a userID or customID for the server SDK to work. See https://docs.statsig.com/messages/serverRequiredUserID for more details.");
    UserData {
        user_id: Some(dyn_value!("")),
        ..UserData::default()
    }
}

#[napi]
impl StatsigUser {
    #[napi(constructor)]
    pub fn new(
        #[napi(
            ts_arg_type = "({userID: string} | {customIDs: Record<string, string> }) & StatsigUserArgs"
        )]
        args: StatsigUserArgs,
    ) -> Self {
        let mut user_data = UserData::default();
        if args.user_id.is_none() && args.custom_ids.is_none() {
            user_data = unidentifiable_user();
        }

        if let Some(user_id) = args.user_id {
            user_data.user_id = Some(dyn_value!(user_id));
        }

        if let Some(custom_ids) = args.custom_ids {
            user_data.custom_ids = Some(Self::convert_custom_ids(custom_ids));
        }

        set_dynamic_value_fields!(
            args,
            user_data,
            email,
            ip,
            user_agent,
            country,
            locale,
            app_version
        );

        user_data.custom = Self::convert_to_dynamic_value_map(args.custom);
        user_data.private_attributes = Self::convert_to_dynamic_value_map(args.private_attributes);

        Self {
            inner: StatsigUserActual {
                data: Arc::new(user_data),
            },
        }
    }

    #[napi(js_name = "withUserID")]
    pub fn with_user_id(user_id: String) -> Self {
        Self {
            inner: StatsigUserActual::with_user_id(user_id),
        }
    }

    #[napi(js_name = "withCustomIDs")]
    pub fn with_custom_ids(
        #[napi(ts_arg_type = "Record<string, string>")] custom_ids: HashMap<
            String,
            Either3<String, f64, i64>,
        >,
    ) -> Self {
        Self {
            inner: StatsigUserActual {
                data: Arc::new(UserData {
                    custom_ids: Some(Self::convert_custom_ids(custom_ids)),
                    ..UserData::default()
                }),
            },
        }
    }

    pub fn as_inner(&self) -> &StatsigUserActual {
        &self.inner
    }

    fn convert_custom_ids(
        custom_ids_arg: HashMap<String, Either3<String, f64, i64>>,
    ) -> HashMap<String, DynamicValue> {
        custom_ids_arg
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    match value {
                        Either3::A(v) => dyn_value!(v),
                        Either3::B(v) => dyn_value!(v),
                        Either3::C(v) => dyn_value!(v),
                    },
                )
            })
            .collect()
    }

    fn convert_to_dynamic_value_map(
        map: Option<HashMap<String, ValidPrimitives>>,
    ) -> Option<HashMap<String, DynamicValue>> {
        let map = match map {
            Some(map) => map,
            _ => return None,
        };

        let mut converted: HashMap<String, DynamicValue> = HashMap::new();

        for (key, value) in map {
            match value {
                Either4::A(value) => converted.insert(key, dyn_value!(value)),
                Either4::B(value) => converted.insert(key, dyn_value!(value)),
                Either4::C(value) => converted.insert(key, dyn_value!(value)),
                Either4::D(value) => converted.insert(key, dyn_value!(value)),
            };
        }

        Some(converted)
    }
}

impl From<StatsigUserActual> for StatsigUser {
    fn from(inner: StatsigUserActual) -> Self {
        StatsigUser { inner }
    }
}

macro_rules! add_hashmap_getter_setter {
    ($field_name:expr, $field_accessor:ident, $setter_name:ident, $ts_arg_type:expr) => {
        #[napi]
        impl StatsigUser {
            #[napi(getter, js_name = $field_name)]
            pub fn $field_accessor(&self) -> Option<HashMap<String, String>> {
                let mut result: HashMap<String, String> = HashMap::new();

                let value_map = match &self.inner.data.$field_accessor {
                    Some(value) => value,
                    _ => return None,
                };

                for (key, value) in value_map {
                    if let Some(dyn_str) = &value.string_value {
                        result.insert(key.to_string(), dyn_str.value.clone());
                    }
                }

                Some(result)
            }

            #[napi(setter, js_name = $field_name, ts_args_type = $ts_arg_type)]
            pub fn $setter_name(&mut self, value: Option<HashMap<String, Value>>) {
                let value = match value {
                    Some(value) => value,
                    _ => {
                        let mut_data = Arc::make_mut(&mut self.inner.data);
                        mut_data.$field_accessor = None;
                        return;
                    }
                };

                let mut converted: HashMap<String, DynamicValue> = HashMap::new();

                for (key, value) in value {
                    if $field_name == "customIDs" && !value.is_string() {
                        log_w!(TAG, "Custom ID '{}' is not a string: {}", key, value);

                        if !value.is_number() {
                            continue;
                        }
                    }

                    converted.insert(key, DynamicValue::from(value));
                }

                let mut_data = Arc::make_mut(&mut self.inner.data);
                mut_data.$field_accessor = Some(converted);
            }
        }
    };
}

macro_rules! add_string_getter_setter {
    ($field_name:expr, $field_accessor:ident, $setter_name:ident) => {
        #[napi]
        impl StatsigUser {
            #[napi(getter, js_name = $field_name)]
            pub fn $field_accessor(&self) -> Option<String> {
                match &self.inner.data.$field_accessor {
                    Some(value) => value.string_value.clone().map(|s| s.value),
                    _ => None,
                }
            }

            #[napi(setter, js_name = $field_name)]
            pub fn $setter_name(&mut self, value: Value) {
                match value {
                    Value::Null => {
                        let mut_data = Arc::make_mut(&mut self.inner.data);
                        mut_data.$field_accessor = None;
                    }
                    _ => {
                        let mut_data = Arc::make_mut(&mut self.inner.data);
                        mut_data.$field_accessor = Some(dyn_value!(value));
                    }
                }
            }
        }
    };
}

add_hashmap_getter_setter!(
    "customIDs",
    custom_ids,
    set_custom_ids,
    "value: Record<string, string> | null"
);
add_hashmap_getter_setter!(
    "custom",
    custom,
    set_custom,
    "value: Record<string, string | number | boolean | Array<string | number | boolean>> | null"
);
add_hashmap_getter_setter!(
    "privateAttributes",
    private_attributes,
    set_private_attributes,
    "value: Record<string, string | number | boolean | Array<string | number | boolean>> | null"
);

add_string_getter_setter!("userID", user_id, set_user_id);
add_string_getter_setter!("email", email, set_email);
add_string_getter_setter!("ip", ip, set_ip);
add_string_getter_setter!("userAgent", user_agent, set_user_agent);
add_string_getter_setter!("country", country, set_country);
add_string_getter_setter!("locale", locale, set_locale);
add_string_getter_setter!("appVersion", app_version, set_app_version);
