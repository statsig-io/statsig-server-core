use crate::compression::zstd_decompression_dict::DictionaryDecoder;
use crate::data_store_interface::{get_data_adapter_dcs_key, DataStoreTrait};
use crate::global_configs::GlobalConfigs;
use crate::id_lists_adapter::{IdList, IdListsUpdateListener};
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::specs_response::spec_types::{SpecsResponseFull, SpecsResponseNoUpdates};
use crate::specs_response::spec_types_encoded::DecodedSpecsResponse;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, SpecsInfo, SpecsSource, SpecsUpdate,
    SpecsUpdateListener, StatsigErr, StatsigRuntime,
};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Serialize)]
pub struct SpecStoreData {
    pub source: SpecsSource,
    pub time_received_at: Option<u64>,
    pub values: SpecsResponseFull,
    pub decompression_dict: Option<DictionaryDecoder>,
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
}

impl SpecStore {
    #[must_use]
    pub fn new(
        sdk_key: &str,
        hashed_sdk_key: String,
        statsig_runtime: Arc<StatsigRuntime>,
        data_store: Option<Arc<dyn DataStoreTrait>>,
    ) -> SpecStore {
        SpecStore {
            hashed_sdk_key,
            data: Arc::new(RwLock::new(SpecStoreData {
                values: SpecsResponseFull::blank(),
                time_received_at: None,
                source: SpecsSource::Uninitialized,
                decompression_dict: None,
                id_lists: HashMap::new(),
            })),
            data_store,
            statsig_runtime,
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            global_configs: GlobalConfigs::get_instance(sdk_key),
        }
    }

    pub fn set_source(&self, source: SpecsSource) {
        if let Ok(mut mut_values) = self.data.write() {
            mut_values.source = source;
            log_d!(TAG, "SpecStore - Source Changed ({:?})", mut_values.source);
        }
    }

    pub fn get_current_values(&self) -> Option<SpecsResponseFull> {
        let data = self.data.read().ok()?;
        let json = serde_json::to_string(&data.values).ok()?;
        serde_json::from_str::<SpecsResponseFull>(&json).ok()
    }

    pub fn set_values(&self, values: SpecsUpdate) -> Result<(), StatsigErr> {
        let full_response: DecodedSpecsResponse<SpecsResponseFull> =
            match self.parse_specs_response(&values) {
                Ok(Some(full)) => full,
                Ok(None) => {
                    self.ops_stats_log_no_update(values.source);
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            };

        if self.are_current_values_newer(&full_response) {
            return Ok(());
        }

        let mut mut_values = match self.data.write() {
            Ok(mut_values) => mut_values,
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock: {}", e);
                return Err(StatsigErr::LockFailure(e.to_string()));
            }
        };

        self.try_update_global_configs(&full_response.specs);

        let now = Utc::now().timestamp_millis() as u64;
        let prev_source = mut_values.source.clone();

        mut_values.values = full_response.specs;
        mut_values.time_received_at = Some(now);
        mut_values.source = values.source.clone();
        mut_values.decompression_dict = full_response.decompression_dict.clone();

        self.try_update_data_store(&mut_values.source, values.data, now);
        self.ops_stats_log_config_propogation_diff(
            mut_values.values.time,
            &mut_values.source,
            &prev_source,
        );

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------- [Private Functions]

impl SpecStore {
    fn parse_specs_response(
        &self,
        values: &SpecsUpdate,
    ) -> Result<Option<DecodedSpecsResponse<SpecsResponseFull>>, StatsigErr> {
        let compression_dict = self
            .data
            .read()
            .map(|data| data.decompression_dict.clone())
            .ok()
            .flatten();
        let full_result = DecodedSpecsResponse::<SpecsResponseFull>::from_slice(
            values.data.as_bytes(),
            compression_dict.as_ref(),
        );
        if let Ok(result) = full_result {
            if result.specs.has_updates {
                return Ok(Some(result));
            }

            return Ok(None);
        }

        let no_updates_result = DecodedSpecsResponse::<SpecsResponseNoUpdates>::from_slice(
            values.data.as_bytes(),
            compression_dict.as_ref(),
        );
        if let Ok(result) = no_updates_result {
            if !result.specs.has_updates {
                return Ok(None);
            }
        }

        let error = full_result.err().map_or_else(
            || StatsigErr::JsonParseError("SpecsResponse".to_string(), "Unknown error".to_string()),
            |e| StatsigErr::JsonParseError("SpecsResponse".to_string(), e.to_string()),
        );

        log_error_to_statsig_and_console!(self.ops_stats, TAG, error);
        Err(error)
    }

    fn ops_stats_log_no_update(&self, source: SpecsSource) {
        log_d!(TAG, "SpecStore - No Updates");
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Increment,
            "config_no_update".to_string(),
            1.0,
            Some(HashMap::from([("source".to_string(), source.to_string())])),
        ));
    }

    fn ops_stats_log_config_propogation_diff(
        &self,
        lcut: u64,
        source: &SpecsSource,
        prev_source: &SpecsSource,
    ) {
        let delay = Utc::now().timestamp_millis() as u64 - lcut;
        log_d!(TAG, "SpecStore - Updated ({:?})", source);

        if *prev_source == SpecsSource::Uninitialized || *prev_source == SpecsSource::Loading {
            return;
        }

        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            "config_propogation_diff".to_string(),
            delay as f64,
            Some(HashMap::from([("source".to_string(), source.to_string())])),
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
    }

    fn try_update_data_store(&self, source: &SpecsSource, data: String, now: u64) {
        if *source != SpecsSource::Network {
            return;
        }

        let data_store = match &self.data_store {
            Some(data_store) => data_store.clone(),
            None => return,
        };

        let hashed_key = self.hashed_sdk_key.clone();
        self.statsig_runtime.spawn(
            "spec_store_update_data_store",
            move |_shutdown_notif| async move {
                let _ = data_store
                    .set(&get_data_adapter_dcs_key(&hashed_key), &data, Some(now))
                    .await;
            },
        );
    }

    fn are_current_values_newer(
        &self,
        full_response: &DecodedSpecsResponse<SpecsResponseFull>,
    ) -> bool {
        let guard = match self.data.read() {
            Ok(guard) => guard,
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
                return false;
            }
        };

        let curr_values = &guard.values;
        let curr_checksum = curr_values.checksum.as_deref().unwrap_or_default();
        let new_checksum = full_response.specs.checksum.as_deref().unwrap_or_default();

        let cached_time_is_newer =
            curr_values.time > 0 && curr_values.time > full_response.specs.time;
        let checksums_match = !curr_checksum.is_empty() && curr_checksum == new_checksum;

        if cached_time_is_newer || checksums_match {
            log_d!(
                    TAG,
                    "Received values for [time: {}, checksum: {}], but currently has values for [time: {}, checksum: {}]. Ignoring values.",
                    full_response.specs.time,
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
        match self.data.read() {
            Ok(data) => SpecsInfo {
                lcut: Some(data.values.time),
                checksum: data.values.checksum.clone(),
                zstd_dict_id: data
                    .decompression_dict
                    .as_ref()
                    .map(|d| d.get_dict_id().to_string()),
                source: data.source.clone(),
            },
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
                SpecsInfo {
                    lcut: None,
                    checksum: None,
                    zstd_dict_id: None,
                    source: SpecsSource::Error,
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
            cmab_configs: None,
            overrides: None,
            override_rules: None,
        }
    }
}
