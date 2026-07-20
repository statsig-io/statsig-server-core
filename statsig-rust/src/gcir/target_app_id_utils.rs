use crate::{
    hashing::HashUtil,
    interned_string::InternedString,
    specs_response::spec_types::{Spec, SpecsResponseFull},
    ClientInitResponseOptions, DynamicValue, HashAlgorithm,
};

pub(crate) fn select_app_id_for_gcir(
    options: &ClientInitResponseOptions,
    dcs_values: &SpecsResponseFull,
    hashing: &HashUtil,
) -> Option<DynamicValue> {
    if let Some(target_app_id) = options.get_target_app_id() {
        return Some(DynamicValue::from(target_app_id.clone()));
    }

    let mut app_id = dcs_values.app_id.as_ref();

    let client_sdk_key = match options.client_sdk_key.as_ref() {
        Some(client_sdk_key) => client_sdk_key,
        None => return app_id.cloned(),
    };

    if let Some(app_id_value) = &dcs_values.sdk_keys_to_app_ids {
        app_id = app_id_value.get(client_sdk_key);
    }

    if let Some(app_id_value) = &dcs_values.hashed_sdk_keys_to_app_ids {
        let hashed_key = &hashing.hash(client_sdk_key, &HashAlgorithm::Djb2);
        app_id = app_id_value.get(hashed_key);
    }

    app_id.cloned()
}

pub(crate) fn should_filter_spec_for_app(
    spec: &Spec,
    app_id: &Option<&DynamicValue>,
    options: &ClientInitResponseOptions,
) -> bool {
    should_filter_config_for_app(spec.target_app_ids.as_ref(), app_id, options)
}

pub(crate) fn should_filter_config_for_app(
    target_app_ids: Option<&Vec<InternedString>>,
    app_id: &Option<&DynamicValue>,
    options: &ClientInitResponseOptions,
) -> bool {
    if options.client_sdk_key.is_none() && options.get_target_app_id().is_none() {
        return false;
    }

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
