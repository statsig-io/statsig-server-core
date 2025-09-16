use crate::{
    hashing::HashUtil,
    interned_string::InternedString,
    specs_response::spec_types::{Spec, SpecsResponseFull},
    ClientInitResponseOptions, DynamicValue, HashAlgorithm,
};

pub(crate) fn select_app_id<'a>(
    options: &'a ClientInitResponseOptions,
    dcs_values: &'a SpecsResponseFull,
    hashing: &HashUtil,
) -> Option<&'a DynamicValue> {
    let mut app_id = dcs_values.app_id.as_ref();

    let client_sdk_key = match options.client_sdk_key.as_ref() {
        Some(client_sdk_key) => client_sdk_key,
        None => return app_id,
    };

    if let Some(app_id_value) = &dcs_values.sdk_keys_to_app_ids {
        app_id = app_id_value.get(client_sdk_key);
    }

    if let Some(app_id_value) = &dcs_values.hashed_sdk_keys_to_app_ids {
        let hashed_key = &hashing.hash(client_sdk_key, &HashAlgorithm::Djb2);
        app_id = app_id_value.get(hashed_key);
    }

    app_id
}

pub(crate) fn should_filter_spec_for_app(
    spec: &Spec,
    app_id: &Option<&DynamicValue>,
    client_sdk_key: &Option<String>,
) -> bool {
    should_filter_config_for_app(spec.target_app_ids.as_ref(), app_id, client_sdk_key)
}

pub(crate) fn should_filter_config_for_app(
    target_app_ids: Option<&Vec<InternedString>>,
    app_id: &Option<&DynamicValue>,
    client_sdk_key: &Option<String>,
) -> bool {
    let _client_sdk_key = match client_sdk_key {
        Some(client_sdk_key) => client_sdk_key,
        None => return false,
    };

    let app_id = match app_id {
        Some(app_id) => app_id,
        None => return false,
    };

    let string_app_id = match app_id.string_value.as_ref() {
        Some(string_app_id) => string_app_id,
        None => return false,
    };

    let target_app_ids = match target_app_ids {
        Some(target_app_ids) => target_app_ids,
        None => return true,
    };

    if !target_app_ids.iter().any(|id| id == &string_app_id.value) {
        return true;
    }
    false
}
