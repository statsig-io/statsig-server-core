use crate::{log_e, networking::NetworkProvider};
use parking_lot::Mutex;
use std::{
    sync::{Arc, Weak},
    time::Duration,
};

lazy_static::lazy_static! {
    static ref INSTANCE: Mutex<Option<Weak<dyn NetworkProvider>>> = Mutex::new(None);
}

const TAG: &str = stringify!(NetworkProviderGlobal);

pub struct NetworkProviderGlobal;

impl NetworkProviderGlobal {
    pub fn try_get() -> Option<Weak<dyn NetworkProvider>> {
        let lock = match INSTANCE.try_lock_for(Duration::from_secs(5)) {
            Some(lock) => lock,
            None => {
                log_e!(
                    TAG,
                    "Failed to get network provider: Failed to lock INSTANCE"
                );
                return None;
            }
        };

        match lock.as_ref() {
            Some(weak) => Some(weak.clone()),
            None => None,
        }
    }

    pub fn set(provider: &Arc<dyn NetworkProvider>) {
        match INSTANCE.try_lock_for(Duration::from_secs(5)) {
            Some(mut instance) => {
                *instance = Some(Arc::downgrade(provider));
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to set network provider: Failed to lock INSTANCE"
                );
            }
        }
    }
}
