use std::{collections::HashMap, io::Cursor};

use prost::Message;
use serde_json::json;

use crate::{
    evaluation::{
        dynamic_returnable::DynamicReturnable, dynamic_string::DynamicString,
        evaluator_value::EvaluatorValue,
    },
    interned_str,
    interned_string::InternedString,
    interned_values::InternedStore,
    log_error_to_statsig_and_console,
    networking::ResponseData,
    observability::{ops_stats::OpsStatsForInstance, sdk_errors_observer::ErrorBoundaryEvent},
    specs_response::{
        explicit_params::ExplicitParameters,
        param_store_types::ParameterStore,
        proto_stream_reader::ProtoStreamReader,
        spec_types::{Condition, Rule, Spec, SpecsResponseFull, SpecsResponsePartial},
        specs_hash_map::{SpecPointer, SpecsHashMap},
        statsig_config_specs::{self as pb, any_value},
    },
    StatsigErr,
};

const TAG: &str = "ProtoSpecs";

pub fn deserialize_protobuf(
    ops_stats: &OpsStatsForInstance,
    current_specs: &SpecsResponseFull, /* Intentionally immutable so we can continue using it if parsing fails */
    next_specs: &mut SpecsResponseFull,
    data: &mut ResponseData,
) -> Result<(), StatsigErr> {
    let mut reader = ProtoStreamReader::new(data);

    if !next_specs.is_empty() {
        // We just verify, rather than doing the reset here. The SpecStore is responsible for resetting the next_specs.
        return Err(StatsigErr::ProtobufParseError(
            "SpecsResponseFull".to_string(),
            "Next specs are not empty".to_string(),
        ));
    }

    loop {
        let proto_msg_bytes = reader.read_next_delimited_proto().map_err(|e| {
            let err = StatsigErr::ProtobufParseError(
                "SpecsEnvelope".to_string(),
                format!("Error reading next delimited proto: {}", e),
            );
            log_error_to_statsig_and_console!(ops_stats, TAG, err);
            err
        })?;

        let env: pb::SpecsEnvelope =
            match prost::Message::decode_length_delimited(proto_msg_bytes.as_ref()) {
                Ok(env) => env,
                Err(e) => {
                    let err: StatsigErr = map_decode_err("SpecsEnvelope", e);
                    log_error_to_statsig_and_console!(ops_stats, TAG, err);
                    continue;
                }
            };

        let envelope_kind = match pb::SpecsEnvelopeKind::try_from(env.kind) {
            Ok(kind) => kind,
            Err(e) => {
                let err: StatsigErr = map_unknown_enum_value("SpecsEnvelopeKind", e);
                log_error_to_statsig_and_console!(ops_stats, TAG, err);
                continue;
            }
        };

        match envelope_kind {
            pb::SpecsEnvelopeKind::Done => return Ok(()),
            pb::SpecsEnvelopeKind::TopLevel => {
                consume_errors(ops_stats, || next_specs.handle_top_level_update(env));
            }
            pb::SpecsEnvelopeKind::FeatureGate => {
                consume_errors(ops_stats, || {
                    next_specs.handle_feature_gate_update(env, current_specs)
                });
            }
            pb::SpecsEnvelopeKind::DynamicConfig => {
                consume_errors(ops_stats, || {
                    next_specs.handle_dynamic_config_update(env, current_specs)
                });
            }
            pb::SpecsEnvelopeKind::LayerConfig => {
                consume_errors(ops_stats, || {
                    next_specs.handle_layer_config_update(env, current_specs)
                });
            }
            pb::SpecsEnvelopeKind::ParamStore => {
                consume_errors(ops_stats, || {
                    next_specs.handle_param_store_update(env, current_specs)
                });
            }
            pb::SpecsEnvelopeKind::Condition => {
                consume_errors(ops_stats, || {
                    next_specs.handle_condition_update(env, current_specs)
                });
            }
            pb::SpecsEnvelopeKind::Deletions => {
                consume_errors(ops_stats, || next_specs.handle_deletions_update(env));
            }
            pb::SpecsEnvelopeKind::Checksums => {
                consume_errors(ops_stats, || next_specs.handle_checksums_update(env));
            }
            pb::SpecsEnvelopeKind::CopyPrev => {
                consume_errors(ops_stats, || {
                    next_specs.handle_copy_prev_update(current_specs)
                });
            }
            pb::SpecsEnvelopeKind::Unknown => {
                return make_proto_parse_error("SpecsEnvelope", "Unknown envelope kind");
            }
        };
    }
}

fn consume_errors<F>(ops_stats: &OpsStatsForInstance, f: F)
where
    F: FnOnce() -> Result<(), StatsigErr>,
{
    if let Err(e) = f() {
        log_error_to_statsig_and_console!(ops_stats, TAG, e);
    }
}

impl SpecsResponseFull {
    fn handle_top_level_update(&mut self, envelope: pb::SpecsEnvelope) -> Result<(), StatsigErr> {
        let envelope_data = validate_envelope_data("TopLevel", envelope.data)?;
        let top_level = pb::SpecsTopLevel::decode(envelope_data)
            .map_err(|e| map_decode_err("SpecsTopLevel", e))?;

        self.populate_top_level_from_envelope(top_level)?;

        Ok(())
    }

    fn handle_feature_gate_update(
        &mut self,
        envelope: pb::SpecsEnvelope,
        existing: &SpecsResponseFull,
    ) -> Result<(), StatsigErr> {
        Self::handle_individual_spec_update(
            "FeatureGate",
            envelope,
            &existing.feature_gates,
            &mut self.feature_gates,
            InternedStore::try_get_preloaded_feature_gate,
        )
    }

    fn handle_dynamic_config_update(
        &mut self,
        envelope: pb::SpecsEnvelope,
        existing: &SpecsResponseFull,
    ) -> Result<(), StatsigErr> {
        Self::handle_individual_spec_update(
            "DynamicConfig",
            envelope,
            &existing.dynamic_configs,
            &mut self.dynamic_configs,
            InternedStore::try_get_preloaded_dynamic_config,
        )
    }

    fn handle_layer_config_update(
        &mut self,
        envelope: pb::SpecsEnvelope,
        existing: &SpecsResponseFull,
    ) -> Result<(), StatsigErr> {
        Self::handle_individual_spec_update(
            "LayerConfig",
            envelope,
            &existing.layer_configs,
            &mut self.layer_configs,
            InternedStore::try_get_preloaded_layer_config,
        )
    }

    fn handle_individual_spec_update(
        tag: &str,
        envelope: pb::SpecsEnvelope,
        exiting_map: &SpecsHashMap,
        new_map: &mut SpecsHashMap,
        preload_fetcher: fn(&InternedString) -> Option<SpecPointer>,
    ) -> Result<(), StatsigErr> {
        let name = InternedString::from_string(envelope.name);

        if let Some(spec) = preload_fetcher(&name) {
            if spec.as_spec_ref().checksum.as_deref() == Some(&envelope.checksum) {
                new_map.insert(name, spec);
                return Ok(());
            }
        }

        if let Some(spec_ptr) = exiting_map.get(&name) {
            if spec_ptr.as_spec_ref().checksum.as_deref() == Some(&envelope.checksum) {
                new_map.insert(name, spec_ptr.clone());
                return Ok(());
            }
        }

        let envelope_data = validate_envelope_data(tag, envelope.data)?;
        let pb_spec = pb::Spec::decode(envelope_data).map_err(|e| map_decode_err(tag, e))?;
        let spec = spec_from_pb(envelope.checksum, pb_spec)?;
        new_map.insert(name, SpecPointer::from_spec(spec));

        Ok(())
    }

    fn populate_top_level_from_envelope(
        &mut self,
        top_level: pb::SpecsTopLevel,
    ) -> Result<(), StatsigErr> {
        let partial = serde_json::from_slice::<SpecsResponsePartial>(&top_level.rest)
            .map_err(|e| map_serde_json_err("SpecsResponsePartial", e))?;

        self.merge_from_partial(partial);

        self.checksum = Some(top_level.checksum);
        self.time = top_level.time;
        self.has_updates = top_level.has_updates;
        self.response_format = Some(top_level.response_format);
        self.company_id = Some(top_level.company_id);

        Ok(())
    }

    fn handle_param_store_update(
        &mut self,
        envelope: pb::SpecsEnvelope,
        existing: &SpecsResponseFull,
    ) -> Result<(), StatsigErr> {
        let name = InternedString::from_string(envelope.name);

        let existing_param_store = existing
            .param_stores
            .as_ref()
            .and_then(|param_stores| param_stores.get(&name));

        if let Some(param_store) = existing_param_store {
            if param_store.checksum.as_deref() == Some(&envelope.checksum) {
                self.param_stores
                    .get_or_insert_with(HashMap::default)
                    .insert(name, param_store.clone());
                return Ok(());
            }
        }

        let envelope_data = validate_envelope_data("ParamStore", envelope.data)?;

        let mut param_store = serde_json::from_slice::<ParameterStore>(envelope_data.get_ref())
            .map_err(|e| map_serde_json_err("ParameterStore", e))?;

        param_store.checksum = Some(InternedString::from_string(envelope.checksum));

        self.param_stores
            .get_or_insert_with(HashMap::default)
            .insert(name, param_store);
        Ok(())
    }

    fn handle_condition_update(
        &mut self,
        envelope: pb::SpecsEnvelope,
        existing: &SpecsResponseFull,
    ) -> Result<(), StatsigErr> {
        let name = InternedString::from_string(envelope.name);

        if let Some(condition) = existing.condition_map.get(&name) {
            if condition.checksum.as_deref() == Some(&envelope.checksum) {
                self.condition_map.insert(name, condition.clone());
                return Ok(());
            }
        }

        let envelope_data = validate_envelope_data("Condition", envelope.data)?;
        let pb_condition =
            pb::Condition::decode(envelope_data).map_err(|e| map_decode_err("Condition", e))?;
        let mut condition = condition_from_pb(pb_condition)?;
        condition.checksum = Some(InternedString::from_string(envelope.checksum));
        self.condition_map.insert(name, condition);

        Ok(())
    }

    fn handle_deletions_update(&mut self, envelope: pb::SpecsEnvelope) -> Result<(), StatsigErr> {
        let envelope_data = validate_envelope_data("Deletions", envelope.data)?;
        let deletions = pb::RulesetsResponseDeletions::decode(envelope_data)
            .map_err(|e| map_decode_err("RulesetsResponseDeletions", e))?;
        self.apply_deletions(deletions);
        Ok(())
    }

    fn handle_checksums_update(&mut self, envelope: pb::SpecsEnvelope) -> Result<(), StatsigErr> {
        let envelope_data = validate_envelope_data("Checksums", envelope.data)?;
        let checksums = pb::RulesetsChecksums::decode(envelope_data)
            .map_err(|e| map_decode_err("RulesetsChecksums", e))?;
        let field_checksums = &checksums.field_checksums;

        let condition_map = sum_checksums(self.condition_map.values().map(checksum_for_condition));
        let dynamic_configs = sum_checksums(self.dynamic_configs.0.values().map(checksum_for_spec));
        let feature_gates = sum_checksums(self.feature_gates.0.values().map(checksum_for_spec));
        let layer_configs = sum_checksums(self.layer_configs.0.values().map(checksum_for_spec));
        let param_stores = sum_checksums(
            self.param_stores
                .as_ref()
                .map(|stores| stores.values().map(checksum_for_param_store))
                .into_iter()
                .flatten(),
        );

        validate_field_checksum("condition_map", field_checksums, condition_map)?;
        validate_field_checksum("dynamic_configs", field_checksums, dynamic_configs)?;
        validate_field_checksum("feature_gates", field_checksums, feature_gates)?;
        validate_field_checksum("layer_configs", field_checksums, layer_configs)?;
        validate_field_checksum("param_stores", field_checksums, param_stores)?;

        Ok(())
    }

    fn handle_copy_prev_update(&mut self, existing: &SpecsResponseFull) -> Result<(), StatsigErr> {
        let partial_value = serde_json::to_value(existing)
            .map_err(|e| map_serde_json_err("SpecsResponsePartial", e))?;
        let partial = serde_json::from_value::<SpecsResponsePartial>(partial_value)
            .map_err(|e| map_serde_json_err("SpecsResponsePartial", e))?;
        self.merge_from_partial(partial);

        self.checksum = existing.checksum.clone();
        self.company_id = existing.company_id.clone();
        self.time = existing.time;
        self.has_updates = existing.has_updates;
        self.response_format = existing.response_format.clone();

        self.dynamic_configs = SpecsHashMap(existing.dynamic_configs.0.clone());
        self.feature_gates = SpecsHashMap(existing.feature_gates.0.clone());
        self.layer_configs = SpecsHashMap(existing.layer_configs.0.clone());
        self.condition_map = existing.condition_map.clone();
        self.param_stores = existing.param_stores.clone();
        Ok(())
    }

    fn apply_deletions(&mut self, deletions: pb::RulesetsResponseDeletions) {
        let pb::RulesetsResponseDeletions {
            dynamic_configs,
            feature_gates,
            layer_configs,
            experiment_to_layer,
            condition_map,
            sdk_configs,
            param_stores,
            cmab_configs,
            override_rules,
            overrides,
        } = deletions;

        remove_interned_from_specs_map(&mut self.dynamic_configs, dynamic_configs);
        remove_interned_from_specs_map(&mut self.feature_gates, feature_gates);
        remove_interned_from_specs_map(&mut self.layer_configs, layer_configs);
        remove_string_from_map(&mut self.experiment_to_layer, experiment_to_layer);
        remove_interned_from_hash(&mut self.condition_map, condition_map);
        remove_string_from_opt_map(&mut self.sdk_configs, sdk_configs);
        remove_interned_from_opt_map(&mut self.param_stores, param_stores);
        remove_string_from_opt_map(&mut self.cmab_configs, cmab_configs);
        remove_string_from_opt_map(&mut self.override_rules, override_rules);
        remove_string_from_opt_map(&mut self.overrides, overrides);
    }
}

fn remove_interned_from_specs_map(map: &mut SpecsHashMap, names: Vec<String>) {
    for name in names {
        map.remove(&InternedString::from_string(name));
    }
}

fn remove_interned_from_hash<V, S: std::hash::BuildHasher>(
    map: &mut HashMap<InternedString, V, S>,
    names: Vec<String>,
) {
    for name in names {
        map.remove(&InternedString::from_string(name));
    }
}

fn remove_string_from_map<V>(map: &mut HashMap<String, V>, names: Vec<String>) {
    for name in names {
        map.remove(&name);
    }
}

fn remove_interned_from_opt_map<V>(
    map: &mut Option<HashMap<InternedString, V>>,
    names: Vec<String>,
) {
    let Some(map) = map.as_mut() else {
        return;
    };
    remove_interned_from_hash(map, names)
}

fn remove_string_from_opt_map<V>(map: &mut Option<HashMap<String, V>>, names: Vec<String>) {
    let Some(map) = map.as_mut() else {
        return;
    };
    remove_string_from_map(map, names);
}

fn validate_field_checksum(
    field: &str,
    field_checksums: &HashMap<String, u64>,
    computed: u64,
) -> Result<(), StatsigErr> {
    let Some(expected) = field_checksums.get(field) else {
        return Err(StatsigErr::ProtobufParseError(
            "proto::RulesetsChecksums".to_string(),
            format!("Missing checksum for {field}"),
        ));
    };

    if *expected != computed {
        return Err(StatsigErr::ProtobufParseError(
            "proto::RulesetsChecksums".to_string(),
            format!("Checksum mismatch for {field}: expected {expected}, got {computed}"),
        ));
    }

    Ok(())
}

fn sum_checksums(checksums: impl Iterator<Item = Option<u32>>) -> u64 {
    checksums.fold(0u64, |acc, checksum| {
        acc.wrapping_add(checksum.unwrap_or_default() as u64)
    })
}

fn checksum_for_condition(condition: &Condition) -> Option<u32> {
    checksum_to_u32(condition.checksum.as_ref())
}

fn checksum_for_spec(pointer: &SpecPointer) -> Option<u32> {
    let spec: &Spec = pointer.as_spec_ref();
    checksum_to_u32(spec.checksum.as_ref())
}

fn checksum_for_param_store(store: &ParameterStore) -> Option<u32> {
    checksum_to_u32(store.checksum.as_ref())
}

fn checksum_to_u32(checksum: Option<&InternedString>) -> Option<u32> {
    let value = checksum?.as_str();
    value.parse::<u32>().ok()
}

fn validate_envelope_data(
    envelope_tag: &str,
    data: Option<Vec<u8>>,
) -> Result<Cursor<Vec<u8>>, StatsigErr> {
    match data {
        Some(data) => Ok(Cursor::new(data)),
        None => Err(StatsigErr::ProtobufParseError(
            "SpecsEnvelope".to_string(),
            format!("No data in {} envelope", envelope_tag),
        )),
    }
}

fn condition_from_pb(v: pb::Condition) -> Result<Condition, StatsigErr> {
    let mut condition = Condition {
        condition_type: condition_type_from_pb(
            pb::ConditionType::try_from(v.condition_type)
                .map_err(|e| map_unknown_enum_value("ConditionType", e))?,
        )?,
        target_value: target_value_from_pb(v.target_value)?,
        operator: match v.operator {
            Some(operator) => Some(operator_from_pb(
                pb::Operator::try_from(operator)
                    .map_err(|e| map_unknown_enum_value("Operator", e))?,
            )?),
            None => None,
        },
        field: v.field.map(DynamicString::from),
        additional_values: additional_values_from_pb(v.additional_values)?,
        id_type: id_type_from_pb_to_dynamic_string(v.id_type)?,
        checksum: None,
    };

    if condition.operator.as_deref() == Some("str_matches") {
        if let Some(ref mut target_value) = condition.target_value {
            target_value.compile_regex();
        }
    }

    Ok(condition)
}

fn additional_values_from_pb(
    additional_values: Option<Vec<u8>>,
) -> Result<Option<HashMap<InternedString, InternedString>>, StatsigErr> {
    let additional_values = match additional_values {
        Some(additional_values) => additional_values,
        None => return Ok(None),
    };

    let map = serde_json::from_slice(&additional_values)
        .map_err(|e| map_serde_json_err("AdditionalValues", e))?;
    Ok(Some(map))
}

fn any_value_to_json_value(
    any_value: Option<pb::AnyValue>,
) -> Result<Option<serde_json::Value>, StatsigErr> {
    let value = match any_value.and_then(|v| v.value) {
        Some(value) => value,
        None => return Ok(None),
    };

    let json_value = match value {
        pb::any_value::Value::BoolValue(value) => serde_json::Value::Bool(value),
        pb::any_value::Value::RawValue(value) => {
            serde_json::from_slice(value.as_ref()).map_err(|e| map_serde_json_err("AnyValue", e))?
        }
        pb::any_value::Value::StringValue(value) => serde_json::Value::String(value),
        pb::any_value::Value::DoubleValue(value) => json!(value),
        pb::any_value::Value::Int64Value(value) => json!(value),
        pb::any_value::Value::Uint64Value(value) => json!(value),
    };

    Ok(Some(json_value))
}

fn target_value_from_pb(
    target_value: Option<pb::AnyValue>,
) -> Result<Option<EvaluatorValue>, StatsigErr> {
    if let Some(any_value) = &target_value {
        if let Some(any_value::Value::RawValue(raw_value)) = &any_value.value {
            if let Some(evaluator_value) =
                InternedStore::try_get_preloaded_evaluator_value(raw_value.as_ref())
            {
                return Ok(Some(evaluator_value));
            }
        }
    }

    match any_value_to_json_value(target_value)? {
        Some(json_value) => {
            let evaluator_value = EvaluatorValue::from_json_value(json_value);
            Ok(Some(evaluator_value))
        }
        None => Ok(None),
    }
}

fn operator_from_pb(operator: pb::Operator) -> Result<InternedString, StatsigErr> {
    match operator {
        pb::Operator::Unknown => Err(StatsigErr::ProtobufParseError(
            "proto::Operator".to_string(),
            "Unknown operator".to_string(),
        )),

        // strict equals
        pb::Operator::Eq => Ok(interned_str!("eq")),
        pb::Operator::Neq => Ok(interned_str!("neq")),

        // numerical comparisons
        pb::Operator::Gt => Ok(interned_str!("gt")),
        pb::Operator::Gte => Ok(interned_str!("gte")),
        pb::Operator::Lte => Ok(interned_str!("lte")),
        pb::Operator::Lt => Ok(interned_str!("lt")),

        // string/array comparisons
        pb::Operator::Any => Ok(interned_str!("any")),
        pb::Operator::None => Ok(interned_str!("none")),
        pb::Operator::StrStartsWithAny => Ok(interned_str!("str_starts_with_any")),
        pb::Operator::StrEndsWithAny => Ok(interned_str!("str_ends_with_any")),
        pb::Operator::StrContainsAny => Ok(interned_str!("str_contains_any")),
        pb::Operator::StrContainsNone => Ok(interned_str!("str_contains_none")),
        pb::Operator::StrMatches => Ok(interned_str!("str_matches")),
        pb::Operator::AnyCaseSensitive => Ok(interned_str!("any_case_sensitive")),
        pb::Operator::NoneCaseSensitive => Ok(interned_str!("none_case_sensitive")),

        // time comparisions
        pb::Operator::Before => Ok(interned_str!("before")),
        pb::Operator::After => Ok(interned_str!("after")),
        pb::Operator::On => Ok(interned_str!("on")),

        // id_lists
        pb::Operator::InSegmentList => Ok(interned_str!("in_segment_list")),
        pb::Operator::NotInSegmentList => Ok(interned_str!("not_in_segment_list")),

        // array comparisons
        pb::Operator::ArrayContainsAny => Ok(interned_str!("array_contains_any")),
        pb::Operator::ArrayContainsNone => Ok(interned_str!("array_contains_none")),
        pb::Operator::ArrayContainsAll => Ok(interned_str!("array_contains_all")),
        pb::Operator::NotArrayContainsAll => Ok(interned_str!("not_array_contains_all")),

        // version comparisons
        pb::Operator::VersionGt => Ok(interned_str!("version_gt")),
        pb::Operator::VersionGte => Ok(interned_str!("version_gte")),
        pb::Operator::VersionLt => Ok(interned_str!("version_lt")),
        pb::Operator::VersionLte => Ok(interned_str!("version_lte")),
        pb::Operator::VersionEq => Ok(interned_str!("version_eq")),
        pb::Operator::VersionNeq => Ok(interned_str!("version_neq")),

        // encoded any
        pb::Operator::EncodedAny => Ok(interned_str!("encoded_any")),
    }
}

fn condition_type_from_pb(condition_type: pb::ConditionType) -> Result<InternedString, StatsigErr> {
    match condition_type {
        pb::ConditionType::Unknown => Err(StatsigErr::ProtobufParseError(
            "proto::ConditionType".to_string(),
            "Unknown condition type".to_string(),
        )),

        pb::ConditionType::CurrentTime => Ok(interned_str!("current_time")),
        pb::ConditionType::Public => Ok(interned_str!("public")),
        pb::ConditionType::FailGate => Ok(interned_str!("fail_gate")),
        pb::ConditionType::PassGate => Ok(interned_str!("pass_gate")),
        pb::ConditionType::UaBased => Ok(interned_str!("ua_based")),
        pb::ConditionType::IpBased => Ok(interned_str!("ip_based")),
        pb::ConditionType::UserField => Ok(interned_str!("user_field")),
        pb::ConditionType::EnvironmentField => Ok(interned_str!("environment_field")),
        pb::ConditionType::UserBucket => Ok(interned_str!("user_bucket")),
        pb::ConditionType::TargetApp => Ok(interned_str!("target_app")),
        pb::ConditionType::UnitId => Ok(interned_str!("unit_id")),
    }
}

fn spec_from_pb(checksum: String, spec: pb::Spec) -> Result<Spec, StatsigErr> {
    let checksum = InternedString::from_string(checksum);
    let entity_type = pb::EntityType::try_from(spec.entity)
        .map_err(|e| map_unknown_enum_value("EntityType", e))?;

    let _type = entity_type.to_legacy_type();

    let mut target_app_ids: Option<Vec<InternedString>> = None;
    if !spec.target_app_ids.is_empty() {
        target_app_ids = Some(
            spec.target_app_ids
                .into_iter()
                .map(InternedString::from_string)
                .collect(),
        );
    }

    let mut fields_used: Option<Vec<InternedString>> = None;
    if !spec.fields_used.is_empty() {
        fields_used = Some(
            spec.fields_used
                .into_iter()
                .map(InternedString::from_string)
                .collect(),
        );
    }

    let spec = Spec {
        checksum: Some(checksum),
        _type,
        salt: InternedString::from_string(spec.salt),
        enabled: spec.enabled,
        rules: rules_from_pb(spec.rules)?,
        id_type: id_type_from_pb(spec.id_type)?,
        explicit_parameters: match spec.explicit_parameters.is_empty() {
            true => None,
            false => Some(ExplicitParameters::from_vec(spec.explicit_parameters)),
        },
        entity: entity_type.to_string_type()?,
        has_shared_params: spec.has_shared_params,
        is_active: spec.is_active,
        version: Some(spec.version),
        target_app_ids,
        forward_all_exposures: spec.forward_all_exposures,
        fields_used,
        default_value: return_value_from_pb(spec.default_value)?,
        use_new_layer_eval: spec.use_new_layer_eval,
    };

    Ok(spec)
}

fn rules_from_pb(rules: Vec<pb::Rule>) -> Result<Vec<Rule>, StatsigErr> {
    rules
        .into_iter()
        .map(|pb_rule| {
            let rule = Rule {
                name: InternedString::from_string(pb_rule.name),
                pass_percentage: pb_rule.pass_percentage as f64,
                id: InternedString::from_string(pb_rule.id),
                salt: pb_rule.salt.map(InternedString::from_string),
                conditions: pb_rule
                    .conditions
                    .into_iter()
                    .map(InternedString::from_string)
                    .collect(),
                id_type: id_type_from_pb_to_dynamic_string(pb_rule.id_type)?,

                group_name: pb_rule.group_name.map(InternedString::from_string),

                config_delegate: pb_rule.config_delegate.map(InternedString::from_string),

                is_experiment_group: pb_rule.is_experiment_group,

                sampling_rate: None,
                return_value: return_value_from_pb(pb_rule.return_value)?,
            };

            Ok(rule)
        })
        .collect::<Result<Vec<Rule>, StatsigErr>>()
}

fn return_value_from_pb(
    return_value: Option<pb::ReturnValue>,
) -> Result<DynamicReturnable, StatsigErr> {
    let return_value = match return_value {
        Some(return_value) => return_value,
        None => return Ok(DynamicReturnable::empty()),
    };

    let return_value = match return_value.value {
        Some(return_value) => return_value,
        None => {
            return Err(StatsigErr::ProtobufParseError(
                "proto::ReturnValue".to_string(),
                "No return value".to_string(),
            ))
        }
    };

    let bytes = match return_value {
        pb::return_value::Value::BoolValue(value) => {
            return Ok(DynamicReturnable::from_bool(value))
        }
        pb::return_value::Value::RawValue(value) => value,
    };

    if let Some(returnable) = InternedStore::try_get_preloaded_returnable(bytes.as_ref()) {
        return Ok(returnable);
    }

    serde_json::from_slice(bytes.as_ref()).map_err(|e| map_serde_json_err("ReturnValue", e))
}

fn id_type_from_pb_to_dynamic_string(
    id_type: Option<pb::IdType>,
) -> Result<DynamicString, StatsigErr> {
    let id_type = match id_type.and_then(|i| i.id_type) {
        Some(id_type) => id_type,
        None => {
            return Ok(DynamicString {
                value: InternedString::empty(),
                lowercased_value: InternedString::empty(),
                hash_value: 0,
            })
        }
    };

    match id_type {
        pb::id_type::IdType::KnownIdType(id_type) => match pb::KnownIdType::try_from(id_type) {
            Ok(pb::KnownIdType::UserId) => Ok(DynamicString::from("userID".to_string())),
            Ok(pb::KnownIdType::StableId) => Ok(DynamicString::from("stableID".to_string())),
            Ok(pb::KnownIdType::Unknown) => Err(StatsigErr::ProtobufParseError(
                "proto::KnownIdType".to_string(),
                "Expected ID type to be known".to_string(),
            )),
            Err(e) => Err(map_unknown_enum_value("KnownIdType", e)),
        },
        pb::id_type::IdType::CustomIdType(id_type) => Ok(DynamicString::from(id_type)),
    }
}

fn id_type_from_pb(id_type: Option<pb::IdType>) -> Result<InternedString, StatsigErr> {
    let id_type = match id_type.and_then(|i| i.id_type) {
        Some(id_type) => id_type,
        None => return Ok(InternedString::empty()),
    };

    match id_type {
        pb::id_type::IdType::KnownIdType(id_type) => match pb::KnownIdType::try_from(id_type) {
            Ok(pb::KnownIdType::UserId) => Ok(interned_str!("userID")),
            Ok(pb::KnownIdType::StableId) => Ok(interned_str!("stableID")),
            Ok(pb::KnownIdType::Unknown) => Err(StatsigErr::ProtobufParseError(
                "proto::KnownIdType".to_string(),
                "Expected ID type to be known".to_string(),
            )),
            Err(e) => Err(map_unknown_enum_value("KnownIdType", e)),
        },
        pb::id_type::IdType::CustomIdType(id_type) => Ok(InternedString::from_string(id_type)),
    }
}

impl pb::EntityType {
    fn to_legacy_type(self) -> InternedString {
        if self == pb::EntityType::EntityFeatureGate
            || self == pb::EntityType::EntityHoldout
            || self == pb::EntityType::EntitySegment
        {
            return interned_str!("feature_gate");
        }

        if self == pb::EntityType::EntityDynamicConfig
            || self == pb::EntityType::EntityAutotune
            || self == pb::EntityType::EntityExperiment
            || self == pb::EntityType::EntityLayer
        {
            interned_str!("dynamic_config")
        } else {
            interned_str!("unknown")
        }
    }

    fn to_string_type(self) -> Result<InternedString, StatsigErr> {
        match self {
            pb::EntityType::EntityFeatureGate => Ok(interned_str!("feature_gate")),
            pb::EntityType::EntityDynamicConfig => Ok(interned_str!("dynamic_config")),
            pb::EntityType::EntityAutotune => Ok(interned_str!("autotune")),
            pb::EntityType::EntityExperiment => Ok(interned_str!("experiment")),
            pb::EntityType::EntityLayer => Ok(interned_str!("layer")),
            pb::EntityType::EntitySegment => Ok(interned_str!("segment")),
            pb::EntityType::EntityHoldout => Ok(interned_str!("holdout")),
            pb::EntityType::EntityUnknown => Err(StatsigErr::ProtobufParseError(
                "proto::EntityType".to_string(),
                "Expected entity type to be known".to_string(),
            )),
        }
    }
}

fn map_decode_err(tag: &str, e: prost::DecodeError) -> StatsigErr {
    StatsigErr::ProtobufParseError(format!("proto::{}", tag), e.to_string())
}

fn map_unknown_enum_value(tag: &str, value: prost::UnknownEnumValue) -> StatsigErr {
    StatsigErr::ProtobufParseError(
        format!("proto::{}", tag),
        format!("Unknown enum value: {}", value),
    )
}

fn map_serde_json_err(tag: &str, e: serde_json::Error) -> StatsigErr {
    StatsigErr::ProtobufParseError(format!("proto::{}", tag), e.to_string())
}

fn make_proto_parse_error(tag: &str, message: &str) -> Result<(), StatsigErr> {
    Err(StatsigErr::ProtobufParseError(
        format!("proto::{}", tag),
        message.to_string(),
    ))
}
