use std::sync::{Arc, Mutex, Weak};

use crate::{log_e, networking::NetworkProvider};

lazy_static::lazy_static! {
    static ref INSTANCE: Mutex<Option<Weak<dyn NetworkProvider>>> = Mutex::new(None);
}

const TAG: &str = stringify!(NetworkProviderGlobal);

pub struct NetworkProviderGlobal;

impl NetworkProviderGlobal {
    pub fn try_get() -> Option<Arc<dyn NetworkProvider>> {
        let lock = match INSTANCE.lock() {
            Ok(lock) => lock,
            Err(e) => {
                log_e!(TAG, "Failed to get network provider: {}", e);
                return None;
            }
        };

        match lock.as_ref() {
            Some(weak) => weak.upgrade(),
            None => None,
        }
    }

    pub fn set(provider: &Arc<dyn NetworkProvider>) {
        match INSTANCE.lock() {
            Ok(mut instance) => {
                *instance = Some(Arc::downgrade(provider));
            }
            Err(e) => {
                log_e!(TAG, "Failed to set network provider: {}", e);
            }
        }
    }
}
