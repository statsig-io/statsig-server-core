use std::collections::HashMap;

use napi::bindgen_prelude::{Either3, Either4};
use napi_derive::napi;
use serde_json::Value;
use sigstat::{log_w, DynamicValue, StatsigUser as StatsigUserActual};

const TAG: &str = "StatsigUserNapi";

type ValidPrimitives = Either4<String, f64, bool, Vec<Value>>;

#[napi(object)]
pub struct StatsigUserArgs {
    #[napi(js_name = "userID")]
    pub user_id: String,
    #[napi(js_name = "customIDs")]
    pub custom_ids: HashMap<String, Either3<String, f64, i64>>,
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
                $inner.$field = Some(DynamicValue::from(value));
            }
        )*
    };
}

#[napi]
impl StatsigUser {
    #[napi(constructor)]
    pub fn new(args: StatsigUserArgs) -> Self {
        let mut inner = StatsigUserActual::with_user_id(args.user_id);

        set_dynamic_value_fields!(
            args,
            inner,
            email,
            ip,
            user_agent,
            country,
            locale,
            app_version
        );

        let mut custom_ids = HashMap::new();
        for (key, value) in args.custom_ids {
            let dyn_value = match value {
                Either3::A(v) => DynamicValue::from(v),
                Either3::B(v) => DynamicValue::from(v),
                Either3::C(v) => DynamicValue::from(v),
            };

            custom_ids.insert(key, dyn_value);
        }
        inner.custom_ids = Some(custom_ids);

        inner.custom = Self::convert_to_dynamic_value_map(args.custom);
        inner.private_attributes = Self::convert_to_dynamic_value_map(args.private_attributes);

        Self { inner }
    }

    #[napi(js_name = "withUserID")]
    pub fn with_user_id(user_id: String) -> Self {
        Self {
            inner: StatsigUserActual::with_user_id(user_id),
        }
    }

    #[napi(js_name = "withCustomIDs")]
    pub fn with_custom_ids(custom_ids: HashMap<String, Value>) -> Self {
        let mut converted: HashMap<String, String> = HashMap::new();

        for (key, value) in custom_ids {
            if let Some(v) = value.as_str() {
                converted.insert(key, v.to_string());
                continue;
            }

            log_w!(TAG, "Custom ID '{}' is not a string: {}", key, value);

            if let Some(v) = value.as_number() {
                converted.insert(key, v.to_string());
            }
        }

        Self {
            inner: StatsigUserActual::with_custom_ids(converted),
        }
    }

    pub fn as_inner(&self) -> &StatsigUserActual {
        &self.inner
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
                Either4::A(value) => converted.insert(key, DynamicValue::from(value)),
                Either4::B(value) => converted.insert(key, DynamicValue::from(value)),
                Either4::C(value) => converted.insert(key, DynamicValue::from(value)),
                Either4::D(value) => converted.insert(key, DynamicValue::from(value)),
            };
        }

        Some(converted)
    }
}

macro_rules! add_hashmap_getter_setter {
    ($field_name:expr, $field_accessor:ident, $setter_name:ident, $ts_arg_type:expr) => {
        #[napi]
        impl StatsigUser {
            #[napi(getter, js_name = $field_name)]
            pub fn $field_accessor(&self) -> Option<HashMap<String, String>> {
                let mut result: HashMap<String, String> = HashMap::new();

                let value_map = match &self.inner.$field_accessor {
                    Some(value) => value,
                    _ => return None,
                };

                for (key, value) in value_map {
                    if let Some(value) = &value.string_value {
                        result.insert(key.to_string(), value.clone());
                    }
                }

                Some(result)
            }

            #[napi(setter, js_name = $field_name, ts_args_type = $ts_arg_type)]
            pub fn $setter_name(&mut self, value: Option<HashMap<String, Value>>) {
                let value = match value {
                    Some(value) => value,
                    _ => {
                        self.inner.$field_accessor = None;
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

                self.inner.$field_accessor = Some(converted);
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
                match &self.inner.$field_accessor {
                    Some(value) => value.string_value.clone(),
                    _ => None,
                }
            }

            #[napi(setter, js_name = $field_name)]
            pub fn $setter_name(&mut self, value: Value) {
                match value {
                    Value::Null => self.inner.$field_accessor = None,
                    _ => self.inner.$field_accessor = Some(value.into()),
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
