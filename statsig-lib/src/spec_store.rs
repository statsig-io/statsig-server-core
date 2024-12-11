use crate::data_store_interface::{get_data_adapter_dcs_key, DataStoreTrait};
use crate::id_lists_adapter::{IdList, IdListsUpdateListener};
use crate::spec_types::{SpecsResponse, SpecsResponseFull};
use crate::{log_d, log_e, SpecsInfo, SpecsSource, SpecsUpdate, SpecsUpdateListener, StatsigErr, StatsigRuntime};
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
    pub statsig_runtime: Option<Arc<StatsigRuntime>>
}

impl SpecsUpdateListener for SpecStore {
    fn did_receive_specs_update(&self, update: SpecsUpdate) -> Result<(), StatsigErr>{
        self.set_values(update)
    }

    fn get_current_specs_info(&self) -> SpecsInfo {
        match self.data.read() {
            Ok(data) => SpecsInfo {
                lcut: Some(data.values.time),
                source: data.source.clone(),
            },
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
                SpecsInfo {
                    lcut: None,
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
        Self::new("", None, None)
    }
}

impl SpecStore {
    pub fn new(hashed_sdk_key: &str, data_store: Option<Arc<dyn DataStoreTrait>>, statsig_runtime: Option<Arc<StatsigRuntime>>) -> SpecStore {
        SpecStore {
            hashed_sdk_key: hashed_sdk_key.to_string(),
            data: Arc::new(RwLock::new(SpecStoreData {
                values: SpecsResponseFull::blank(),
                time_received_at: None,
                source: SpecsSource::Uninitialized,
                id_lists: HashMap::new(),
            })),
            data_store,
            statsig_runtime
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
                    log_d!(TAG, "SpecStore - No Updates");
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
                    log_d!(TAG, "SpecStore - No Updates");
                }
                return Ok(());
            }
            Err(e) => {
                // todo: Handle bad parsing
                log_e!(TAG, "{:?}, {:?}", e, values.source);
                return Err(StatsigErr::JsonParseError("config_spec".to_string(), e.to_string()));
            }
        };

        if let Ok(mut mut_values) = self.data.write() {
            if mut_values.values.time > 0 && mut_values.values.time > dcs.time {
                log_d!(
                    TAG, "SpecStore - Received values for {}, but currently has values for {}. Ignoring values.",
                    dcs.time,
                    mut_values.values.time
                );
                return Ok(());
            }
            let curr_time =Some(Utc::now().timestamp_millis() as u64);
            mut_values.values = *dcs;
            mut_values.time_received_at = curr_time;
            mut_values.source = values.source.clone();
            if self.data_store.is_some() && mut_values.source == SpecsSource::Network {
                match self.data_store.clone() {
                    Some(data_store) =>  {
                        let hashed_key = self.hashed_sdk_key.clone();
                        self.statsig_runtime.clone().map(
                            |rt| {
                                let copy = curr_time.clone();
                                rt.spawn("update data adapter",     move |_|  async move {
                                    let _ = data_store.set(&get_data_adapter_dcs_key(&hashed_key), &values.data, copy).await;
                                })
                            }
                        );
                    }
                    None => {},
                }
            }

            log_d!(TAG, "SpecStore - Updated ({:?})", mut_values.source);
        }

        Ok(())
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
            default_environment: None,
            app_id: None,
            sdk_keys_to_app_ids: None,
            hashed_sdk_keys_to_app_ids: None,
        }
    }
}
