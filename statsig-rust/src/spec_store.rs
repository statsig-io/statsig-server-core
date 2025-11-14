use crate::data_store_interface::{get_data_adapter_dcs_key, DataStoreTrait};
use crate::evaluation::evaluator::SpecType;
use crate::global_configs::GlobalConfigs;
use crate::id_lists_adapter::{IdList, IdListsUpdateListener};
use crate::interned_string::InternedString;
use crate::networking::ResponseData;
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_event_emitter::{SdkEvent, SdkEventEmitter};
use crate::specs_response::spec_types::{SpecsResponseFull, SpecsResponseNoUpdates};
use crate::utils::maybe_trim_malloc;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, read_lock_or_else, SpecsInfo, SpecsSource,
    SpecsUpdate, SpecsUpdateListener, StatsigErr, StatsigRuntime,
};
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct SpecStoreData {
    pub source: SpecsSource,
    pub source_api: Option<String>,
    pub time_received_at: Option<u64>,
    pub values: SpecsResponseFull,
    pub next_values: Option<SpecsResponseFull>,
    pub id_lists: HashMap<String, IdList>,
}

const TAG: &str = stringify!(SpecStore);

pub struct SpecStore {
    pub data: Arc<RwLock<SpecStoreData>>,

    hashed_sdk_key: String,
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
        hashed_sdk_key: String,
        statsig_runtime: Arc<StatsigRuntime>,
        event_emitter: Arc<SdkEventEmitter>,
        data_store: Option<Arc<dyn DataStoreTrait>>,
    ) -> SpecStore {
        SpecStore {
            hashed_sdk_key,
            data: Arc::new(RwLock::new(SpecStoreData {
                values: SpecsResponseFull::default(),
                next_values: Some(SpecsResponseFull::default()),
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
        match self.data.try_write_for(Duration::from_secs(5)) {
            Some(mut data) => {
                data.source = source;
                log_d!(TAG, "Source Changed ({:?})", data.source);
            }
            None => {
                log_e!(TAG, "Failed to acquire write lock: Failed to lock data");
            }
        }
    }

    pub fn get_current_values(&self) -> Option<SpecsResponseFull> {
        let data = match self.data.try_read_for(Duration::from_secs(5)) {
            Some(data) => data,
            None => {
                log_e!(TAG, "Failed to acquire read lock: Failed to lock data");
                return None;
            }
        };
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
        };

        let entity_name = InternedString::from_str_ref(entity_name);
        let entity = entities.get(&entity_name);

        match entity {
            Some(entity) => match &entity.inner.fields_used {
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
            .filter(|(_, v)| v.inner.entity == entity_type)
            .map(|(k, _)| k.unperformant_to_string())
            .collect()
    }

    pub fn set_values(&self, specs_update: SpecsUpdate) -> Result<(), StatsigErr> {
        let mut specs_update = specs_update;
        let mut next_values = match self.data.try_write_for(Duration::from_secs(5)) {
            Some(mut data) => data.next_values.take().unwrap_or_default(),
            None => {
                log_e!(TAG, "Failed to acquire write lock: Failed to lock data");
                return Err(StatsigErr::LockFailure(
                    "Failed to acquire write lock: Failed to lock data".to_string(),
                ));
            }
        };

        match self.parse_specs_response(&mut specs_update, &mut next_values) {
            Ok(ParseResult::HasUpdates) => (),
            Ok(ParseResult::NoUpdates) => {
                self.ops_stats_log_no_update(specs_update.source, specs_update.source_api);
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        };

        if self.are_current_values_newer(&next_values) {
            return Ok(());
        }

        self.try_update_global_configs(&next_values);

        let now = Utc::now().timestamp_millis() as u64;
        let (prev_source, prev_lcut, curr_values_time) = self.swap_current_with_next(
            next_values,
            &specs_update,
            now,
            specs_update.source_api.clone(),
        )?;

        self.try_update_data_store(&specs_update.source, specs_update.data, now);
        self.ops_stats_log_config_propagation_diff(
            curr_values_time,
            prev_lcut,
            &specs_update.source,
            &prev_source,
            specs_update.source_api,
        );

        // Glibc requested more memory than needed when deserializing a big json blob
        // And memory allocator fails to return it.
        // To prevent service from OOMing, manually unused heap memory.
        maybe_trim_malloc();

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------- [Private Functions]

impl SpecStore {
    fn parse_specs_response(
        &self,
        values: &mut SpecsUpdate,
        next_values: &mut SpecsResponseFull,
    ) -> Result<ParseResult, StatsigErr> {
        let parse_result = values.data.deserialize_in_place(next_values);

        if parse_result.is_ok() && next_values.has_updates {
            return Ok(ParseResult::HasUpdates);
        }

        let no_updates_result = values.data.deserialize_into::<SpecsResponseNoUpdates>();
        if let Ok(result) = no_updates_result {
            if !result.has_updates {
                return Ok(ParseResult::NoUpdates);
            }
        }

        let error = parse_result.err().unwrap_or_else(|| {
            StatsigErr::JsonParseError("SpecsResponse".to_string(), "Unknown error".to_string())
        });

        log_error_to_statsig_and_console!(self.ops_stats, TAG, error);
        Err(error)
    }

    fn swap_current_with_next(
        &self,
        next_values: SpecsResponseFull,
        specs_update: &SpecsUpdate,
        now: u64,
        source_api: Option<String>,
    ) -> Result<(SpecsSource, u64, u64), StatsigErr> {
        match self.data.try_write_for(Duration::from_secs(5)) {
            Some(mut data) => {
                let prev_source = std::mem::replace(&mut data.source, specs_update.source.clone());
                let prev_lcut = data.values.time;

                let mut temp = next_values;
                std::mem::swap(&mut data.values, &mut temp);
                data.next_values = Some(temp);

                data.time_received_at = Some(now);
                data.source_api = source_api;

                self.emit_specs_updated_sdk_event(&data.source, &data.source_api, &data.values);

                Ok((prev_source, prev_lcut, data.values.time))
            }
            None => {
                log_e!(TAG, "Failed to acquire write lock: Failed to lock data");
                Err(StatsigErr::LockFailure(
                    "Failed to acquire write lock: Failed to lock data".to_string(),
                ))
            }
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

    fn ops_stats_log_no_update(&self, source: SpecsSource, source_api: Option<String>) {
        log_d!(TAG, "No Updates");
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Increment,
            "config_no_update".to_string(),
            1.0,
            Some(HashMap::from([
                ("source".to_string(), source.to_string()),
                (
                    "spec_source_api".to_string(),
                    source_api.unwrap_or_default(),
                ),
            ])),
        ));
    }

    fn ops_stats_log_config_propagation_diff(
        &self,
        lcut: u64,
        prev_lcut: u64,
        source: &SpecsSource,
        prev_source: &SpecsSource,
        source_api: Option<String>,
    ) {
        let delay = Utc::now().timestamp_millis() as u64 - lcut;
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
                (
                    "spec_source_api".to_string(),
                    source_api.unwrap_or_default(),
                ),
            ])),
        ));
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

        let hashed_key = self.hashed_sdk_key.clone();

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
                    .set(
                        &get_data_adapter_dcs_key(&hashed_key),
                        &data_string,
                        Some(now),
                    )
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

    fn are_current_values_newer(&self, next_values: &SpecsResponseFull) -> bool {
        let data = match self.data.try_read_for(Duration::from_secs(5)) {
            Some(data) => data,
            None => {
                log_e!(TAG, "Failed to acquire read lock: Failed to lock data");
                return false;
            }
        };

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

// -------------------------------------------------------------------------------------------- [Impl SpecsUpdateListener]

impl SpecsUpdateListener for SpecStore {
    fn did_receive_specs_update(&self, update: SpecsUpdate) -> Result<(), StatsigErr> {
        self.set_values(update)
    }

    fn get_current_specs_info(&self) -> SpecsInfo {
        match self.data.try_read_for(Duration::from_secs(5)) {
            Some(data) => SpecsInfo {
                lcut: Some(data.values.time),
                checksum: data.values.checksum.clone(),
                source: data.source.clone(),
                source_api: data.source_api.clone(),
            },
            None => {
                log_e!(TAG, "Failed to acquire read lock: Failed to lock data");
                SpecsInfo {
                    lcut: None,
                    checksum: None,
                    source: SpecsSource::Error,
                    source_api: None,
                }
            }
        }
    }
}

// -------------------------------------------------------------------------------------------- [Impl IdListsUpdateListener]

impl IdListsUpdateListener for SpecStore {
    fn get_current_id_list_metadata(
        &self,
    ) -> HashMap<String, crate::id_lists_adapter::IdListMetadata> {
        match self.data.try_read_for(Duration::from_secs(5)) {
            Some(data) => data
                .id_lists
                .iter()
                .map(|(key, list)| (key.clone(), list.metadata.clone()))
                .collect(),
            None => {
                log_e!(TAG, "Failed to acquire read lock: Failed to lock data");
                HashMap::new()
            }
        }
    }

    fn did_receive_id_list_updates(
        &self,
        updates: HashMap<String, crate::id_lists_adapter::IdListUpdate>,
    ) {
        let mut data = match self.data.try_write_for(Duration::from_secs(5)) {
            Some(data) => data,
            None => {
                log_e!(TAG, "Failed to acquire write lock: Failed to lock data");
                return;
            }
        };

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

enum ParseResult {
    HasUpdates,
    NoUpdates,
}
