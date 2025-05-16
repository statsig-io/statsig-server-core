use crate::compression::zstd_decompression_dict::DictionaryDecoder;
use crate::data_store_interface::{get_data_adapter_dcs_key, DataStoreTrait};
use crate::global_configs::GlobalConfigs;
use crate::id_lists_adapter::{IdList, IdListsUpdateListener};
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::specs_response::spec_types::{SpecsResponseFull, SpecsResponseNoUpdates};
use crate::specs_response::spec_types_encoded::DecodedSpecsResponse;
use crate::utils::maybe_trim_malloc;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, SpecsInfo, SpecsSource, SpecsUpdate,
    SpecsUpdateListener, StatsigErr, StatsigRuntime,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct SpecStoreData {
    pub source: SpecsSource,
    pub source_api: Option<String>,
    pub time_received_at: Option<u64>,
    pub values: SpecsResponseFull,
    pub next_values: Option<SpecsResponseFull>,
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
                values: SpecsResponseFull::default(),
                next_values: Some(SpecsResponseFull::default()),
                time_received_at: None,
                source: SpecsSource::Uninitialized,
                source_api: None,
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
            log_d!(TAG, "Source Changed ({:?})", mut_values.source);
        }
    }

    pub fn get_current_values(&self) -> Option<SpecsResponseFull> {
        let data = self.data.read().ok()?;
        let json = serde_json::to_string(&data.values).ok()?;
        serde_json::from_str::<SpecsResponseFull>(&json).ok()
    }

    pub fn set_values(&self, specs_update: SpecsUpdate) -> Result<(), StatsigErr> {
        let (mut next_values, decompression_dict) = match self.data.write() {
            Ok(mut data) => (
                data.next_values.take().unwrap_or_default(),
                data.decompression_dict.clone(),
            ),
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock: {}", e);
                return Err(StatsigErr::LockFailure(e.to_string()));
            }
        };

        let decompression_dict =
            match self.parse_specs_response(&specs_update, &mut next_values, decompression_dict) {
                Ok(Some(full)) => full,
                Ok(None) => {
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
            decompression_dict,
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
        values: &SpecsUpdate,
        next_values: &mut SpecsResponseFull,
        decompression_dict: Option<DictionaryDecoder>,
    ) -> Result<Option<Option<DictionaryDecoder>>, StatsigErr> {
        let full_update_decoder_result = DecodedSpecsResponse::from_slice(
            &values.data,
            next_values,
            decompression_dict.as_ref(),
        );

        if let Ok(result) = full_update_decoder_result {
            if next_values.has_updates {
                return Ok(Some(result));
            }

            return Ok(None);
        }

        let mut next_no_updates = SpecsResponseNoUpdates { has_updates: false };
        let no_updates_decoder_result = DecodedSpecsResponse::from_slice(
            &values.data,
            &mut next_no_updates,
            decompression_dict.as_ref(),
        );

        if no_updates_decoder_result.is_ok() && !next_no_updates.has_updates {
            return Ok(None);
        }

        let error = full_update_decoder_result.err().map_or_else(
            || StatsigErr::JsonParseError("SpecsResponse".to_string(), "Unknown error".to_string()),
            |e| StatsigErr::JsonParseError("SpecsResponse".to_string(), e.to_string()),
        );

        log_error_to_statsig_and_console!(self.ops_stats, TAG, error);
        Err(error)
    }

    fn swap_current_with_next(
        &self,
        next_values: SpecsResponseFull,
        specs_update: &SpecsUpdate,
        decompression_dict: Option<DictionaryDecoder>,
        now: u64,
        source_api: Option<String>,
    ) -> Result<(SpecsSource, u64, u64), StatsigErr> {
        match self.data.write() {
            Ok(mut data) => {
                let prev_source = std::mem::replace(&mut data.source, specs_update.source.clone());
                let prev_lcut = data.values.time;

                let mut temp = next_values;
                std::mem::swap(&mut data.values, &mut temp);
                data.next_values = Some(temp);

                data.time_received_at = Some(now);
                data.decompression_dict = decompression_dict;
                data.source_api = source_api;
                Ok((prev_source, prev_lcut, data.values.time))
            }
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock: {}", e);
                Err(StatsigErr::LockFailure(e.to_string()))
            }
        }
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
    }

    fn try_update_data_store(&self, source: &SpecsSource, data: Vec<u8>, now: u64) {
        if source != &SpecsSource::Network {
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
                let data_string = match String::from_utf8(data) {
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
    }

    fn are_current_values_newer(&self, next_values: &SpecsResponseFull) -> bool {
        let data = match self.data.read() {
            Ok(data) => data,
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
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
        match self.data.read() {
            Ok(data) => SpecsInfo {
                lcut: Some(data.values.time),
                checksum: data.values.checksum.clone(),
                zstd_dict_id: data
                    .decompression_dict
                    .as_ref()
                    .map(|d| d.get_dict_id().to_string()),
                source: data.source.clone(),
                source_api: data.source_api.clone(),
            },
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
                SpecsInfo {
                    lcut: None,
                    checksum: None,
                    zstd_dict_id: None,
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
