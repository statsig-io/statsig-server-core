use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::data_store_interface::DataStoreTrait;
use crate::evaluation::evaluator::SpecType;
use crate::global_configs::GlobalConfigs;
use crate::id_lists_adapter::{IdList, IdListsUpdateListener};
use crate::interned_string::InternedString;
use crate::networking::ResponseData;
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_event_emitter::{SdkEvent, SdkEventEmitter};
use crate::specs_response::proto_specs::deserialize_protobuf;
use crate::specs_response::spec_types::{SpecsResponseFull, SpecsResponseNoUpdates};
use crate::utils::try_release_unused_heap_memory;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, read_lock_or_else, write_lock_or_else,
    SpecsFormat, SpecsInfo, SpecsSource, SpecsUpdate, SpecsUpdateListener, StatsigErr,
    StatsigOptions, StatsigRuntime,
};

pub struct SpecStoreData {
    pub source: SpecsSource,
    pub source_api: Option<String>,
    pub time_received_at: Option<u64>,
    pub values: SpecsResponseFull,
    pub id_lists: HashMap<String, IdList>,
}

const TAG: &str = stringify!(SpecStore);

pub struct SpecStore {
    pub data: Arc<RwLock<SpecStoreData>>,

    data_store_key: String,
    data_store: Option<Arc<dyn DataStoreTrait>>,
    statsig_runtime: Arc<StatsigRuntime>,
    ops_stats: Arc<OpsStatsForInstance>,
    global_configs: Arc<GlobalConfigs>,
    event_emitter: Arc<SdkEventEmitter>,
}

impl SpecStore {
    #[must_use]
    pub fn new(
        sdk_key: &str,
        data_store_key: String,
        statsig_runtime: Arc<StatsigRuntime>,
        event_emitter: Arc<SdkEventEmitter>,
        options: Option<&StatsigOptions>,
    ) -> SpecStore {
        let mut data_store = None;
        if let Some(options) = options {
            data_store = options.data_store.clone();
        }

        SpecStore {
            data_store_key,
            data: Arc::new(RwLock::new(SpecStoreData {
                values: SpecsResponseFull::default(),
                time_received_at: None,
                source: SpecsSource::Uninitialized,
                source_api: None,
                id_lists: HashMap::new(),
            })),
            event_emitter,
            data_store,
            statsig_runtime,
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            global_configs: GlobalConfigs::get_instance(sdk_key),
        }
    }

    pub fn set_source(&self, source: SpecsSource) {
        let mut locked_data = write_lock_or_else!(self.data, {
            log_e!(TAG, "Failed to acquire write lock: Failed to lock data");
            return;
        });

        locked_data.source = source;
        log_d!(TAG, "Source Changed ({:?})", locked_data.source);
    }

    pub fn get_current_values(&self) -> Option<SpecsResponseFull> {
        let data = read_lock_or_else!(self.data, {
            log_e!(TAG, "Failed to acquire read lock: Failed to lock data");
            return None;
        });

        let json = serde_json::to_string(&data.values).ok()?;
        serde_json::from_str::<SpecsResponseFull>(&json).ok()
    }

    pub fn get_fields_used_for_entity(
        &self,
        entity_name: &str,
        entity_type: SpecType,
    ) -> Vec<String> {
        let data = read_lock_or_else!(self.data, {
            log_error_to_statsig_and_console!(
                &self.ops_stats,
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        let entities = match entity_type {
            SpecType::Gate => &data.values.feature_gates,
            SpecType::DynamicConfig | SpecType::Experiment => &data.values.dynamic_configs,
            SpecType::Layer => &data.values.layer_configs,
            SpecType::ParameterStore => return vec![],
        };

        let entity_name = InternedString::from_str_ref(entity_name);
        let entity = entities.get(&entity_name);

        match entity {
            Some(entity) => match &entity.as_spec_ref().fields_used {
                Some(fields) => fields.iter().map(|f| f.unperformant_to_string()).collect(),
                None => vec![],
            },
            None => vec![],
        }
    }

    pub fn unperformant_keys_entity_filter(
        &self,
        top_level_key: &str,
        entity_type: &str,
    ) -> Vec<String> {
        let data = read_lock_or_else!(self.data, {
            log_error_to_statsig_and_console!(
                &self.ops_stats,
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        if top_level_key == "param_stores" {
            match &data.values.param_stores {
                Some(param_stores) => {
                    return param_stores
                        .keys()
                        .map(|k| k.unperformant_to_string())
                        .collect()
                }
                None => return vec![],
            }
        }

        let values = match top_level_key {
            "feature_gates" => &data.values.feature_gates,
            "dynamic_configs" => &data.values.dynamic_configs,
            "layer_configs" => &data.values.layer_configs,
            _ => {
                log_e!(TAG, "Invalid top level key: {}", top_level_key);
                return vec![];
            }
        };

        if entity_type == "*" {
            return values.keys().map(|k| k.unperformant_to_string()).collect();
        }

        values
            .iter()
            .filter(|(_, v)| v.as_spec_ref().entity == entity_type)
            .map(|(k, _)| k.unperformant_to_string())
            .collect()
    }

    pub fn set_values(&self, mut specs_update: SpecsUpdate) -> Result<(), StatsigErr> {
        // Updating the spec store is a three step process that interacts with the SpecStoreData lock:
        // 1. Prep (Read Lock). Deserialize the new data and compare it to the current values.
        // 2. Apply (Write Lock). Update the spec store with the new values.
        // 3. Notify (Read Lock). Emit the SDK event and update the data store.

        // --- Prep ---

        let prep_result = self.specs_update_prep(&mut specs_update).map_err(|e| {
            log_error_to_statsig_and_console!(self.ops_stats, TAG, e);
            e
        })?;

        let (next_values, response_format) = match prep_result {
            PrepResult::HasUpdates(next_values, response_format) => (next_values, response_format),
            PrepResult::CurrentValuesNewer => return Ok(()),
            PrepResult::NoUpdates => {
                self.ops_stats_log_no_update(specs_update.source, specs_update.source_api);
                return Ok(());
            }
        };

        // --- Apply ---

        let apply_result = self
            .specs_update_apply(next_values, &specs_update)
            .map_err(|e| {
                log_error_to_statsig_and_console!(self.ops_stats, TAG, e);
                e
            })?;

        try_release_unused_heap_memory();

        // --- Notify ---

        self.specs_update_notify(response_format, specs_update, apply_result)
            .map_err(|e| {
                log_error_to_statsig_and_console!(self.ops_stats, TAG, e);
                e
            })?;

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------- [ Private ]

enum PrepResult {
    HasUpdates(Box<SpecsResponseFull>, SpecsFormat),
    NoUpdates,
    CurrentValuesNewer,
}

struct ApplyResult {
    prev_source: SpecsSource,
    prev_lcut: u64,
    time_received_at: u64,
}

impl SpecStore {
    fn specs_update_prep(&self, specs_update: &mut SpecsUpdate) -> Result<PrepResult, StatsigErr> {
        let response_format = self.get_spec_response_format(specs_update);

        let read_data = read_lock_or_else!(self.data, {
            let msg = "Failed to acquire read lock for extract_response_from_update";
            log_e!(TAG, "{}", msg);
            return Err(StatsigErr::LockFailure(msg.to_string()));
        });

        let current_values = &read_data.values;

        // First, try a Full Specs Response deserialization
        let first_deserialize_result =
            self.deserialize_specs_data(current_values, &response_format, &mut specs_update.data);

        let first_deserialize_error = match first_deserialize_result {
            Ok(next_values) => {
                if self.are_current_values_newer(&read_data, &next_values) {
                    return Ok(PrepResult::CurrentValuesNewer);
                }

                if next_values.has_updates {
                    return Ok(PrepResult::HasUpdates(
                        Box::new(next_values),
                        response_format,
                    ));
                }

                None
            }
            Err(e) => Some(e),
        };

        // Second, try a No Updates deserialization
        let second_deserialize_result = specs_update
            .data
            .deserialize_into::<SpecsResponseNoUpdates>();

        let second_deserialize_error = match second_deserialize_result {
            Ok(result) => {
                if !result.has_updates {
                    return Ok(PrepResult::NoUpdates);
                }

                None
            }
            Err(e) => Some(e),
        };

        let error = first_deserialize_error
            .or(second_deserialize_error)
            .unwrap_or_else(|| {
                StatsigErr::JsonParseError("SpecsResponse".to_string(), "Unknown error".to_string())
            });

        Err(error)
    }

    fn specs_update_apply(
        &self,
        next_values: Box<SpecsResponseFull>,
        specs_update: &SpecsUpdate,
    ) -> Result<ApplyResult, StatsigErr> {
        // DANGER: try_update_global_configs contains its own locks
        self.try_update_global_configs(&next_values);

        let mut data = write_lock_or_else!(self.data, {
            let msg = "Failed to acquire write lock for swap_current_with_next";
            log_e!(TAG, "{}", msg);
            return Err(StatsigErr::LockFailure(msg.to_string()));
        });

        let prev_source = std::mem::replace(&mut data.source, specs_update.source.clone());
        let prev_lcut = data.values.time;
        let time_received_at = Utc::now().timestamp_millis() as u64;

        data.values = *next_values;
        data.time_received_at = Some(time_received_at);
        data.source_api = specs_update.source_api.clone();

        Ok(ApplyResult {
            prev_source,
            prev_lcut,
            time_received_at,
        })
    }

    fn specs_update_notify(
        &self,
        response_format: SpecsFormat,
        specs_update: SpecsUpdate,
        apply_result: ApplyResult,
    ) -> Result<(), StatsigErr> {
        let current_lcut = {
            let read_lock = read_lock_or_else!(self.data, {
                let msg = "Failed to acquire read lock for set_values";
                log_e!(TAG, "{}", msg);
                return Err(StatsigErr::LockFailure(msg.to_string()));
            });

            self.emit_specs_updated_sdk_event(
                &read_lock.source,
                &read_lock.source_api,
                &read_lock.values,
            );

            read_lock.values.time
        };

        if let SpecsFormat::Json = response_format {
            // protobuf response writes to data store are not current supported
            self.try_update_data_store(
                &specs_update.source,
                specs_update.data,
                apply_result.time_received_at,
            );
        }

        self.ops_stats_log_config_propagation_diff(
            current_lcut,
            apply_result.prev_lcut,
            &specs_update.source,
            &apply_result.prev_source,
            specs_update.source_api,
            response_format,
        );

        Ok(())
    }

    fn deserialize_specs_data(
        &self,
        current_values: &SpecsResponseFull,
        response_format: &SpecsFormat,
        response_data: &mut ResponseData,
    ) -> Result<SpecsResponseFull, StatsigErr> {
        let mut next_values = SpecsResponseFull::default();

        let parse_result = match response_format {
            SpecsFormat::Protobuf => deserialize_protobuf(
                &self.ops_stats,
                current_values,
                &mut next_values,
                response_data,
            ),
            SpecsFormat::Json => response_data.deserialize_in_place(&mut next_values),
        };

        match parse_result {
            Ok(()) => Ok(next_values),
            Err(e) => Err(e),
        }
    }

    fn emit_specs_updated_sdk_event(
        &self,
        source: &SpecsSource,
        source_api: &Option<String>,
        values: &SpecsResponseFull,
    ) {
        self.event_emitter.emit(SdkEvent::SpecsUpdated {
            source,
            source_api,
            values,
        });
    }

    fn get_spec_response_format(&self, update: &SpecsUpdate) -> SpecsFormat {
        let content_type = update.data.get_header_ref("content-type");
        if content_type.map(|s| s.as_str().contains("application/octet-stream")) != Some(true) {
            return SpecsFormat::Json;
        }

        let content_encoding = update.data.get_header_ref("content-encoding");
        if content_encoding.map(|s| s.as_str().contains("statsig-br")) != Some(true) {
            return SpecsFormat::Json;
        }

        SpecsFormat::Protobuf
    }

    fn try_update_global_configs(&self, dcs: &SpecsResponseFull) {
        if let Some(diagnostics) = &dcs.diagnostics {
            self.global_configs
                .set_diagnostics_sampling_rates(diagnostics.clone());
        }

        if let Some(sdk_configs) = &dcs.sdk_configs {
            self.global_configs.set_sdk_configs(sdk_configs.clone());
        }

        if let Some(sdk_flags) = &dcs.sdk_flags {
            self.global_configs.set_sdk_flags(sdk_flags.clone());
        }
    }

    fn try_update_data_store(&self, source: &SpecsSource, mut data: ResponseData, now: u64) {
        if source != &SpecsSource::Network {
            return;
        }

        let data_store = match &self.data_store {
            Some(data_store) => data_store.clone(),
            None => return,
        };

        let data_store_key = self.data_store_key.clone();

        let spawn_result = self.statsig_runtime.spawn(
            "spec_store_update_data_store",
            move |_shutdown_notif| async move {
                let data_string = match data.read_to_string() {
                    Ok(s) => s,
                    Err(e) => {
                        log_e!(TAG, "Failed to convert data to string: {}", e);
                        return;
                    }
                };

                let _ = data_store
                    .set(&data_store_key, &data_string, Some(now))
                    .await;
            },
        );

        if let Err(e) = spawn_result {
            log_e!(
                TAG,
                "Failed to spawn spec store update data store task: {e}"
            );
        }
    }

    fn are_current_values_newer(
        &self,
        data: &SpecStoreData,
        next_values: &SpecsResponseFull,
    ) -> bool {
        let curr_values = &data.values;
        let curr_checksum = curr_values.checksum.as_deref().unwrap_or_default();
        let new_checksum = next_values.checksum.as_deref().unwrap_or_default();

        let cached_time_is_newer = curr_values.time > 0 && curr_values.time > next_values.time;
        let checksums_match = !curr_checksum.is_empty() && curr_checksum == new_checksum;

        if cached_time_is_newer || checksums_match {
            log_d!(
                TAG,
                "Received values for [time: {}, checksum: {}], but currently has values for [time: {}, checksum: {}]. Ignoring values.",
                next_values.time,
                new_checksum,
                curr_values.time,
                curr_checksum,
                );
            return true;
        }

        false
    }
}

// -------------------------------------------------------------------------------------------- [ OpsStats Helpers ]

impl SpecStore {
    fn ops_stats_log_no_update(&self, source: SpecsSource, source_api: Option<String>) {
        log_d!(TAG, "No Updates");
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Increment,
            "config_no_update".to_string(),
            1.0,
            Some(HashMap::from([
                ("source".to_string(), source.to_string()),
                ("source_api".to_string(), source_api.unwrap_or_default()),
            ])),
        ));
    }

    #[allow(clippy::too_many_arguments)]
    fn ops_stats_log_config_propagation_diff(
        &self,
        lcut: u64,
        prev_lcut: u64,
        source: &SpecsSource,
        prev_source: &SpecsSource,
        source_api: Option<String>,
        response_format: SpecsFormat,
    ) {
        let delay = (Utc::now().timestamp_millis() as u64).saturating_sub(lcut);
        log_d!(TAG, "Updated ({:?})", source);

        if *prev_source == SpecsSource::Uninitialized || *prev_source == SpecsSource::Loading {
            return;
        }

        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            "config_propagation_diff".to_string(),
            delay as f64,
            Some(HashMap::from([
                ("source".to_string(), source.to_string()),
                ("lcut".to_string(), lcut.to_string()),
                ("prev_lcut".to_string(), prev_lcut.to_string()),
                ("source_api".to_string(), source_api.unwrap_or_default()),
                (
                    "response_format".to_string(),
                    Into::<&str>::into(&response_format).to_string(),
                ),
            ])),
        ));
    }
}

// -------------------------------------------------------------------------------------------- [Impl SpecsUpdateListener]

impl SpecsUpdateListener for SpecStore {
    fn did_receive_specs_update(&self, update: SpecsUpdate) -> Result<(), StatsigErr> {
        self.set_values(update)
    }

    fn get_current_specs_info(&self) -> SpecsInfo {
        let data = read_lock_or_else!(self.data, {
            log_e!(
                TAG,
                "Failed to acquire read lock for get_current_specs_info"
            );
            return SpecsInfo {
                lcut: None,
                checksum: None,
                source: SpecsSource::Error,
                source_api: None,
            };
        });

        SpecsInfo {
            lcut: Some(data.values.time),
            checksum: data.values.checksum.clone(),
            source: data.source.clone(),
            source_api: data.source_api.clone(),
        }
    }
}

// -------------------------------------------------------------------------------------------- [Impl IdListsUpdateListener]

impl IdListsUpdateListener for SpecStore {
    fn get_current_id_list_metadata(
        &self,
    ) -> HashMap<String, crate::id_lists_adapter::IdListMetadata> {
        let data = read_lock_or_else!(self.data, {
            let err = StatsigErr::LockFailure(
                "Failed to acquire read lock for id list metadata".to_string(),
            );
            log_error_to_statsig_and_console!(self.ops_stats, TAG, err);
            return HashMap::new();
        });

        data.id_lists
            .iter()
            .map(|(key, list)| (key.clone(), list.metadata.clone()))
            .collect()
    }

    fn did_receive_id_list_updates(
        &self,
        updates: HashMap<String, crate::id_lists_adapter::IdListUpdate>,
    ) {
        let mut data = write_lock_or_else!(self.data, {
            let err = StatsigErr::LockFailure(
                "Failed to acquire write lock for did_receive_id_list_updates".to_string(),
            );
            log_error_to_statsig_and_console!(self.ops_stats, TAG, err);

            return;
        });

        // delete any id_lists that are not in the updates
        data.id_lists.retain(|name, _| updates.contains_key(name));

        for (list_name, update) in updates {
            if let Some(entry) = data.id_lists.get_mut(&list_name) {
                // update existing
                entry.apply_update(update);
            } else {
                // add new
                let mut list = IdList::new(update.new_metadata.clone());
                list.apply_update(update);
                data.id_lists.insert(list_name, list);
            }
        }
    }
}
