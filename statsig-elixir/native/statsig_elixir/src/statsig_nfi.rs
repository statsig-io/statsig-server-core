use rustler::{Env, Error, ResourceArc, Term};
use statsig_rust::{statsig_types::Layer as LayerActual, Statsig};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    statsig_options_nfi::StatsigOptions,
    statsig_types_nfi::{AllowedPrimitive, DynamicConfig, Experiment, FeatureGate},
    statsig_user_nfi::StatsigUser,
};
use serde_json::Value;

struct StatsigResource {
    pub statsig_core: RwLock<Arc<Statsig>>,
}

#[allow(non_local_definitions)]
fn load(env: Env, _: Term) -> bool {
    _ = rustler::resource!(StatsigResource, env);
    _ = rustler::resource!(LayerResource, env);
    true
}

pub struct LayerResource {
    pub core: RwLock<Arc<LayerActual>>,
}

impl LayerResource {
    pub fn new(layer: LayerActual) -> Self {
        LayerResource {
            core: RwLock::new(Arc::new(layer)),
        }
    }
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn new(
    sdk_key: String,
    options: Option<StatsigOptions>,
) -> Result<ResourceArc<StatsigResource>, Error> {
    let statsig = Statsig::new(&sdk_key, options.map(|op| Arc::new(op.into())));
    Ok(ResourceArc::new(StatsigResource {
        statsig_core: RwLock::new(Arc::new(statsig)),
    }))
}

#[rustler::nif]
pub fn initialize(statsig: ResourceArc<StatsigResource>) -> Result<(), Error> {
    match statsig.statsig_core.read() {
        Ok(read) => {
            let statsig: Arc<Statsig> = Arc::clone(&read);
            let rt_handle = match statsig.statsig_runtime.get_handle() {
                Ok(handle) => handle,
                Err(_) => return Err(Error::RaiseAtom("Failed to get Statsig")),
            };

            match rt_handle.block_on(statsig.initialize()) {
                Ok(_) => Ok(()),
                Err(_) => Err(Error::RaiseAtom("failed to init")),
            }
        }
        Err(_) => Err(Error::RaiseAtom("failed to init")),
    }
}

#[rustler::nif]
pub fn get_feature_gate(
    statsig: ResourceArc<StatsigResource>,
    gate_name: &str,
    statsig_user: StatsigUser,
) -> Result<FeatureGate, Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => Ok(read_guard
            .get_feature_gate(&statsig_user.into(), gate_name)
            .into()),
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn check_gate(
    statsig: ResourceArc<StatsigResource>,
    gate_name: &str,
    statsig_user: StatsigUser,
) -> Result<bool, Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => Ok(read_guard.check_gate(&statsig_user.into(), gate_name)),
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn get_config(
    statsig: ResourceArc<StatsigResource>,
    config_name: &str,
    statsig_user: StatsigUser,
) -> Result<DynamicConfig, Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => Ok(read_guard
            .get_dynamic_config(&statsig_user.into(), config_name)
            .into()),
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn get_experiment(
    statsig: ResourceArc<StatsigResource>,
    experiment_name: &str,
    statsig_user: StatsigUser,
) -> Result<Experiment, Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => Ok(read_guard
            .get_experiment(&statsig_user.into(), experiment_name)
            .into()),
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn get_layer(
    statsig: ResourceArc<StatsigResource>,
    layer_name: &str,
    statsig_user: StatsigUser,
) -> Result<ResourceArc<LayerResource>, Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => {
            let layer = read_guard.get_layer(&statsig_user.into(), layer_name);
            Ok(ResourceArc::new(LayerResource::new(layer)))
        }
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn log_event(
    statsig: ResourceArc<StatsigResource>,
    statsig_user: StatsigUser,
    event_name: &str,
    value: Option<&str>,
    metadata: Option<HashMap<String, String>>,
) -> Result<(), Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => {
            read_guard.log_event(
                &statsig_user.into(),
                event_name,
                value.map(|v| v.to_string()),
                metadata,
            );
            Ok(())
        }
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn log_event_with_number(
    statsig: ResourceArc<StatsigResource>,
    statsig_user: StatsigUser,
    event_name: &str,
    value: Option<f64>,
    metadata: Option<HashMap<String, String>>,
) -> Result<(), Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => {
            read_guard.log_event_with_number(&statsig_user.into(), event_name, value, metadata);
            Ok(())
        }
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn flush(statsig: ResourceArc<StatsigResource>) -> Result<(), Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => {
            let statsig: Arc<Statsig> = Arc::clone(&read_guard);

            let rt_handle = match statsig.statsig_runtime.get_handle() {
                Ok(handle) => handle,
                Err(_) => return Err(Error::RaiseAtom("Failed to get Statsig")),
            };

            rt_handle.block_on(async move { statsig.flush_events().await });

            Ok(())
        }
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn shutdown(statsig: ResourceArc<StatsigResource>) -> Result<(), Error> {
    match statsig.statsig_core.read() {
        Ok(read_guard) => {
            let statsig: Arc<Statsig> = Arc::clone(&read_guard);

            let rt_handle = match statsig.statsig_runtime.get_handle() {
                Ok(handle) => handle,
                Err(_) => return Err(Error::RaiseAtom("Failed to get Statsig")),
            };

            rt_handle.block_on(async move {
                match statsig.shutdown().await {
                    Ok(_) => Ok(()),
                    Err(_) => Err(Error::RaiseAtom("Failed to shutdown")),
                }
            })
        }
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

// Layer NFI
#[rustler::nif]
pub fn layer_get(
    layer: ResourceArc<LayerResource>,
    param_name: &str,
    default: AllowedPrimitive,
) -> Result<AllowedPrimitive, Error> {
    match layer.core.read() {
        Ok(layer) => match default.clone() {
            AllowedPrimitive::Int(i) => Ok(AllowedPrimitive::Int(layer.get_i64(param_name, i))),
            AllowedPrimitive::Float(f) => Ok(AllowedPrimitive::Float(layer.get_f64(param_name, f))),
            AllowedPrimitive::Bool(b) => Ok(AllowedPrimitive::Bool(layer.get_bool(param_name, b))),
            AllowedPrimitive::Str(_) => match layer.get_raw_value(param_name) {
                Some(json_value) => {
                    let res = match json_value {
                        Value::String(s) => s.to_owned(),
                        _ => json_value.to_string(),
                    };
                    Ok(AllowedPrimitive::Str(res))
                }
                None => Ok(default),
            },
        },
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn layer_get_rule_id(layer: ResourceArc<LayerResource>) -> Result<String, Error> {
    match layer.core.read() {
        Ok(read_guard) => Ok(read_guard.rule_id.clone()),
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

#[rustler::nif]
pub fn layer_get_group_name(layer: ResourceArc<LayerResource>) -> Result<Option<String>, Error> {
    match layer.core.read() {
        Ok(read_guard) => Ok(read_guard.group_name.clone()),
        Err(_) => Err(Error::RaiseAtom("Failed to get Statsig")),
    }
}

rustler::init!("Elixir.Statsig.NativeBindings", load = load);
