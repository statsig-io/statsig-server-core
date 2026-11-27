use crate::{Amplitude, AmplitudeError, Event};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

pub type PluginFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type FlushResults = Vec<Result<u16, AmplitudeError>>;

#[derive(Clone, Debug)]
pub struct DeliveryResult {
    pub event: Event,
    pub code: u16,
    pub message: String,
}

impl DeliveryResult {
    pub(crate) fn status(self) -> Result<u16, AmplitudeError> {
        if self.code == 200 || (self.code == 0 && self.message.is_empty()) {
            Ok(self.code)
        } else {
            Err(AmplitudeError::Plugin {
                code: self.code,
                message: self.message,
            })
        }
    }
}

pub enum Delivery {
    Ready(DeliveryResult),
    Pending(PluginFuture<'static, Result<DeliveryResult, AmplitudeError>>),
}

impl Delivery {
    pub fn pending(
        future: impl Future<Output = Result<DeliveryResult, AmplitudeError>> + Send + 'static,
    ) -> Self {
        Self::Pending(Box::pin(future))
    }

    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    pub async fn wait(self) -> Result<DeliveryResult, AmplitudeError> {
        match self {
            Self::Ready(result) => Ok(result),
            Self::Pending(future) => future.await,
        }
    }
}

pub trait EventPlugin: Send + Sync {
    fn setup(&self, _client: &Amplitude) -> Result<(), AmplitudeError> {
        Ok(())
    }

    /// Return an event to continue processing, or None to drop it.
    fn execute(&self, event: Event) -> Option<Event>;
}

pub trait DestinationPlugin: Send + Sync {
    fn setup(&self, _client: &Amplitude) -> Result<(), AmplitudeError> {
        Ok(())
    }

    /// Start delivery during this call. A pending delivery observes the outcome.
    fn execute(&self, event: Event) -> Result<Delivery, AmplitudeError>;

    fn flush(&self) -> PluginFuture<'_, FlushResults> {
        Box::pin(async { Vec::new() })
    }

    fn shutdown(&self) -> PluginFuture<'_, FlushResults> {
        self.flush()
    }
}

#[derive(Clone)]
pub enum Plugin {
    Before(Arc<dyn EventPlugin>),
    Enrichment(Arc<dyn EventPlugin>),
    Destination(Arc<dyn DestinationPlugin>),
}

impl Plugin {
    pub(crate) fn setup(&self, client: &Amplitude) -> Result<(), AmplitudeError> {
        match self {
            Self::Before(plugin) | Self::Enrichment(plugin) => plugin.setup(client),
            Self::Destination(plugin) => plugin.setup(client),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginId(u64);

#[derive(Default)]
struct State {
    registrations: Vec<(PluginId, Plugin)>,
    next_id: u64,
    closed: bool,
}

pub struct Timeline {
    opt_out: bool,
    state: Mutex<State>,
}

impl Timeline {
    pub fn new(opt_out: bool) -> Self {
        Self {
            opt_out,
            state: Mutex::default(),
        }
    }

    pub fn add(&self, plugin: Plugin) -> Result<PluginId, AmplitudeError> {
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return Err(AmplitudeError::Closed);
        }
        let id = PluginId(state.next_id);
        state.next_id += 1;
        state.registrations.push((id, plugin));
        Ok(id)
    }

    pub fn remove(&self, id: PluginId) -> bool {
        let removed = {
            let mut state = self.state.lock().unwrap();
            state
                .registrations
                .iter()
                .position(|(current, _)| *current == id)
                .map(|position| state.registrations.remove(position))
        };
        removed.is_some()
    }

    pub fn apply(&self, mut event: Event) -> Result<Delivery, AmplitudeError> {
        let plugins = {
            let state = self.state.lock().unwrap();
            if state.closed {
                return Err(AmplitudeError::Closed);
            }
            state.registrations.clone()
        };
        if self.opt_out {
            return Ok(no_delivery(event, 0, ""));
        }

        let processors = plugins
            .iter()
            .filter_map(|(_, plugin)| match plugin {
                Plugin::Before(plugin) => Some(plugin),
                _ => None,
            })
            .chain(plugins.iter().filter_map(|(_, plugin)| match plugin {
                Plugin::Enrichment(plugin) => Some(plugin),
                _ => None,
            }));
        for plugin in processors {
            match plugin.execute(event.clone()) {
                Some(processed) => event = processed,
                None => return Ok(no_delivery(event, 0, "")),
            }
        }

        let mut deliveries = plugins.iter().filter_map(|(_, plugin)| match plugin {
            Plugin::Destination(destination) => Some(destination.execute(event.clone())),
            _ => None,
        });
        let first = deliveries.next();
        for _ in deliveries {}
        first.unwrap_or_else(|| {
            Ok(no_delivery(
                event,
                100,
                "Event not tracked, no destination plugins on the instance",
            ))
        })
    }

    pub async fn flush(&self) -> Vec<FlushResults> {
        let destinations = {
            let state = self.state.lock().unwrap();
            destinations(&state)
        };
        let mut results = Vec::new();
        for destination in destinations {
            results.push(destination.flush().await);
        }
        results
    }

    pub async fn shutdown(&self) -> Vec<FlushResults> {
        let destinations = {
            let mut state = self.state.lock().unwrap();
            state.closed = true;
            destinations(&state)
        };
        let mut results = Vec::new();
        for destination in destinations {
            results.push(destination.shutdown().await);
        }
        results
    }
}

fn destinations(state: &State) -> Vec<Arc<dyn DestinationPlugin>> {
    state
        .registrations
        .iter()
        .filter_map(|(_, plugin)| match plugin {
            Plugin::Destination(destination) => Some(Arc::clone(destination)),
            _ => None,
        })
        .collect()
}

fn no_delivery(event: Event, code: u16, message: &str) -> Delivery {
    Delivery::Ready(DeliveryResult {
        event,
        code,
        message: message.into(),
    })
}
