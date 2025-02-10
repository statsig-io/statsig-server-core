use crate::data_store_interface::{get_data_adapter_dcs_key, DataStoreTrait};
use crate::id_lists_adapter::{IdList, IdListsUpdateListener};
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::OpsStatsForInstance;
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::spec_types::{SpecsResponse, SpecsResponseFull};
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, DynamicValue, SpecsInfo, SpecsSource,
    SpecsUpdate, SpecsUpdateListener, StatsigErr, StatsigRuntime,
};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Serialize)]
pub struct SpecStoreData {
    pub source: SpecsSource,
    pub time_received_at: Option<u64>,
    pub values: SpecsResponseFull,

    pub id_lists: HashMap<String, IdList>,
}

const TAG: &str = stringify!(SpecStore);

pub struct SpecStore {
    pub hashed_sdk_key: String,
    pub data: Arc<RwLock<SpecStoreData>>,
    pub data_store: Option<Arc<dyn DataStoreTrait>>,
    pub statsig_runtime: Option<Arc<StatsigRuntime>>,
    ops_stats: Arc<OpsStatsForInstance>,
}

impl SpecsUpdateListener for SpecStore {
    fn did_receive_specs_update(&self, update: SpecsUpdate) -> Result<(), StatsigErr> {
        self.set_values(update)
    }

    fn get_current_specs_info(&self) -> SpecsInfo {
        match self.data.read() {
            Ok(data) => SpecsInfo {
                lcut: Some(data.values.time),
                checksum: data.values.checksum.clone(),
                source: data.source.clone(),
            },
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
                SpecsInfo {
                    lcut: None,
                    checksum: None,
                    source: SpecsSource::Error,
                }
            }
        }
    }
}

impl IdListsUpdateListener for SpecStore {
    fn get_current_id_list_metadata(
        &self,
    ) -> HashMap<String, crate::id_lists_adapter::IdListMetadata> {
        match self.data.read() {
            Ok(data) => data
                .id_lists
                .iter()
                .map(|(key, list)| (key.clone(), list.metadata.clone()))
                .collect(),
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
                HashMap::new()
            }
        }
    }

    fn did_receive_id_list_updates(
        &self,
        updates: HashMap<String, crate::id_lists_adapter::IdListUpdate>,
    ) {
        let mut data = match self.data.write() {
            Ok(data) => data,
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock: {}", e);
                return;
            }
        };

        // delete any id_lists that are not in the updates
        data.id_lists.retain(|name, _| updates.contains_key(name));

        for (list_name, update) in updates {
            if let Some(entry) = data.id_lists.get_mut(&list_name) {
                // update existing
                entry.apply_update(&update);
            } else {
                // add new
                let mut list = IdList::new(update.new_metadata.clone());
                list.apply_update(&update);
                data.id_lists.insert(list_name, list);
            }
        }
    }
}

impl Default for SpecStore {
    fn default() -> Self {
        Self::new(
            "".to_string(),
            None,
            None,
            Arc::new(OpsStatsForInstance::new()),
        )
    }
}

impl SpecStore {
    pub fn new(
        hashed_sdk_key: String,
        data_store: Option<Arc<dyn DataStoreTrait>>,
        statsig_runtime: Option<Arc<StatsigRuntime>>,
        ops_stats: Arc<OpsStatsForInstance>,
    ) -> SpecStore {
        SpecStore {
            hashed_sdk_key,
            data: Arc::new(RwLock::new(SpecStoreData {
                values: SpecsResponseFull::blank(),
                time_received_at: None,
                source: SpecsSource::Uninitialized,
                id_lists: HashMap::new(),
            })),
            data_store,
            statsig_runtime,
            ops_stats,
        }
    }

    pub fn set_source(&self, source: SpecsSource) {
        if let Ok(mut mut_values) = self.data.write() {
            mut_values.source = source;
            log_d!(TAG, "SpecStore - Source Changed ({:?})", mut_values.source);
        }
    }

    pub fn set_values(&self, values: SpecsUpdate) -> Result<(), StatsigErr> {
        let parsed = serde_json::from_str::<SpecsResponse>(&values.data);
        let dcs = match parsed {
            Ok(SpecsResponse::Full(full)) => {
                if !full.has_updates {
                    self.log_no_update(values.source);
                    return Ok(());
                }

                log_d!(
                    TAG,
                    "SpecStore Full Update: {} - [gates({}), configs({}), layers({})]",
                    full.time,
                    full.feature_gates.len(),
                    full.dynamic_configs.len(),
                    full.layer_configs.len(),
                );

                full
            }
            Ok(SpecsResponse::NoUpdates(no_updates)) => {
                if !no_updates.has_updates {
                    self.log_no_update(values.source);
                    return Ok(());
                }
                log_error_to_statsig_and_console!(
                    self.ops_stats,
                    TAG,
                    "Empty response with has_updates = true {:?}",
                    values.source
                );
                return Err(StatsigErr::JsonParseError(
                    "SpecsResponse".to_owned(),
                    "Parse failure. 'has_update' is true, but failed to deserialize to response format 'dcs-v2'".to_owned(),
                ));
            }
            Err(e) => {
                // todo: Handle bad parsing
                log_error_to_statsig_and_console!(
                    self.ops_stats,
                    TAG,
                    "{:?}, {:?}",
                    e,
                    values.source
                );
                return Err(StatsigErr::JsonParseError(
                    "config_spec".to_string(),
                    e.to_string(),
                ));
            }
        };
        if let Ok(mut mut_values) = self.data.write() {
            let cached_time_is_newer =
                mut_values.values.time > 0 && mut_values.values.time > dcs.time;
            let checksums_match =
                mut_values
                    .values
                    .checksum
                    .as_ref()
                    .is_some_and(|cached_checksum| {
                        dcs.checksum
                            .as_ref()
                            .is_some_and(|new_checksum| cached_checksum == new_checksum)
                    });

            if cached_time_is_newer || checksums_match {
                log_d!(
                    TAG,
                    "SpecStore - Received values for [time: {}, checksum: {}], but currently has values for [time: {}, checksum: {}]. Ignoring values.",
                    dcs.time,
                    dcs.checksum.unwrap_or("".to_string()),
                    mut_values.values.time,
                    mut_values.values.checksum.clone().unwrap_or("".to_string()),
                );
                return Ok(());
            }
            let curr_time = Some(Utc::now().timestamp_millis() as u64);
            let prev_source = mut_values.source.clone();
            mut_values.values = *dcs;
            mut_values.time_received_at = curr_time;
            mut_values.source = values.source.clone();
            if self.data_store.is_some() && mut_values.source == SpecsSource::Network {
                if let Some(data_store) = self.data_store.clone() {
                    let hashed_key = self.hashed_sdk_key.clone();
                    self.statsig_runtime.clone().map(|rt| {
                        let copy = curr_time;
                        rt.spawn("update data adapter", move |_| async move {
                            let _ = data_store
                                .set(&get_data_adapter_dcs_key(&hashed_key), &values.data, copy)
                                .await;
                        })
                    });
                }
            }
            self.log_processing_config(
                mut_values.values.time,
                mut_values.source.clone(),
                prev_source,
            );
        }

        Ok(())
    }

    pub fn get_sdk_config_value(&self, key: &str) -> Option<DynamicValue> {
        match self.data.read() {
            Ok(data) => match &data.values.sdk_configs {
                Some(sdk_configs) => sdk_configs.get(key).cloned(),
                None => {
                    log_d!(TAG, "SDK Configs not found");
                    None
                }
            },
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
                None
            }
        }
    }

    fn log_processing_config(&self, lcut: u64, source: SpecsSource, prev_source: SpecsSource) {
        let delay = Utc::now().timestamp_millis() as u64 - lcut;
        log_d!(TAG, "SpecStore - Updated ({:?})", source);

        if prev_source != SpecsSource::Uninitialized && prev_source != SpecsSource::Loading {
            self.ops_stats.log(ObservabilityEvent::new_event(
                MetricType::Dist,
                "config_propogation_diff".to_string(),
                delay as f64,
                Some(HashMap::from([("source".to_string(), source.to_string())])),
            ));
        }
    }

    fn log_no_update(&self, source: SpecsSource) {
        log_d!(TAG, "SpecStore - No Updates");
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Increment,
            "config_no_update".to_string(),
            1.0,
            Some(HashMap::from([("source".to_string(), source.to_string())])),
        ));
    }
}

impl SpecsResponseFull {
    fn blank() -> Self {
        SpecsResponseFull {
            feature_gates: Default::default(),
            dynamic_configs: Default::default(),
            layer_configs: Default::default(),
            condition_map: Default::default(),
            experiment_to_layer: Default::default(),
            has_updates: true,
            time: 0,
            checksum: None,
            default_environment: None,
            app_id: None,
            sdk_keys_to_app_ids: None,
            hashed_sdk_keys_to_app_ids: None,
            diagnostics: None,
            param_stores: None,
            sdk_configs: None,
        }
    }
}
