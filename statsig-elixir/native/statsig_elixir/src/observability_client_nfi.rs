use rustler::{env::OwnedEnv, types::atom::Atom, types::local_pid::LocalPid, Encoder, Env, Term};
use statsig_rust::{log_d, log_w, ObservabilityClient, OpsStatsEventObserver};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::data_store_nfi::{current_managed_env, ManagedEnvGuard};

const TAG: &str = "[ObservabilityClient NFI] ";

mod atoms {
    rustler::atoms! {
        observability_client_request = "statsig_observability_client_request",
        init,
        increment,
        gauge,
        dist,
        error,
        no_payload
    }
}

#[derive(rustler::NifStruct)]
#[module = "Statsig.ObservabilityClient.Reference"]
pub struct StatsigObservabilityClientReference {
    pub pid: LocalPid,
    pub high_cardinality_tags: Option<Vec<String>>,
}

// Bridges the core's ObservabilityClient callbacks to an Elixir process. The
// metric-emitting callbacks (init/increment/gauge/dist/error) are fire-and-
// forget messages, so no reply channel is required.
pub struct ElixirObservabilityClient {
    pid: LocalPid,
    // Static allowlist answered from Rust: the synchronous
    // should_enable_high_cardinality_for_this_tag callback cannot round-trip
    // over the fire-and-forget bridge, but a fixed tag list needs no round trip.
    high_cardinality_tags: Option<Vec<String>>,
    send_failure_warned: AtomicBool,
}

impl ElixirObservabilityClient {
    pub fn new(pid: LocalPid, high_cardinality_tags: Option<Vec<String>>) -> Self {
        Self {
            pid,
            high_cardinality_tags,
            send_failure_warned: AtomicBool::new(false),
        }
    }

    fn send<F>(&self, build: F)
    where
        F: for<'a> FnOnce(Env<'a>) -> Term<'a>,
    {
        let sent = if let Some(env) = current_managed_env() {
            env.send(&self.pid, build(env)).is_ok()
        } else {
            let mut owned_env = OwnedEnv::new();
            owned_env.send_and_clear(&self.pid, build).is_ok()
        };

        if !sent {
            // Warn once so a dead bridge process is noticeable at default log
            // levels, then drop to debug to avoid per-metric spam.
            if !self.send_failure_warned.swap(true, Ordering::Relaxed) {
                log_w!(
                    TAG,
                    "Failed to message Elixir observability client; the bridge process may have exited. Further metrics will be dropped."
                );
            } else {
                log_d!(TAG, "Failed to message Elixir observability client");
            }
        }
    }

    fn send_metric(
        &self,
        action: Atom,
        metric_name: String,
        value: f64,
        tags: Option<HashMap<String, String>>,
    ) {
        self.send(move |env| {
            (
                atoms::observability_client_request(),
                action,
                (metric_name, value, tags),
            )
                .encode(env)
        });
    }
}

impl ObservabilityClient for ElixirObservabilityClient {
    fn init(&self) {
        self.send(|env| {
            (
                atoms::observability_client_request(),
                atoms::init(),
                atoms::no_payload(),
            )
                .encode(env)
        });
    }

    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.send_metric(atoms::increment(), metric_name, value, tags);
    }

    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.send_metric(atoms::gauge(), metric_name, value, tags);
    }

    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.send_metric(atoms::dist(), metric_name, value, tags);
    }

    fn error(&self, tag: String, error: String) {
        self.send(move |env| {
            (
                atoms::observability_client_request(),
                atoms::error(),
                (tag, error),
            )
                .encode(env)
        });
    }

    // Answered from the static list configured at start_link (no Elixir round
    // trip). When no list is configured, None defers to the core's default
    // (high-cardinality tags such as lcut are dropped), matching Node.
    fn should_enable_high_cardinality_for_this_tag(&self, tag: String) -> Option<bool> {
        self.high_cardinality_tags
            .as_ref()
            .map(|tags| tags.contains(&tag))
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }
}

// Test-only helper that drives the bridge the same way the core would, so
// tests can assert the Elixir process receives the expected messages without
// standing up a full Statsig instance. Mirrors the C FFI's
// __internal__test_observability_client. For "error", `tags["error"]` supplies
// the error string. Returns the callback result for
// "should_enable_high_cardinality_for_this_tag" (nil otherwise).
#[rustler::nif]
#[allow(non_snake_case)]
pub fn __internal__test_observability_client(
    env: Env<'_>,
    reference: StatsigObservabilityClientReference,
    action: String,
    metric_name: String,
    value: f64,
    tags: Option<HashMap<String, String>>,
) -> Option<bool> {
    // Route sends through the NIF's own env, since OwnedEnv cannot be used from
    // a managed BEAM thread.
    let _guard = ManagedEnvGuard::new(env);
    let client = ElixirObservabilityClient::new(reference.pid, reference.high_cardinality_tags);
    match action.as_str() {
        "init" => client.init(),
        "increment" => client.increment(metric_name, value, tags),
        "gauge" => client.gauge(metric_name, value, tags),
        "dist" => client.dist(metric_name, value, tags),
        "error" => {
            let error = tags
                .as_ref()
                .and_then(|t| t.get("error").cloned())
                .unwrap_or_default();
            client.error(metric_name, error);
        }
        "should_enable_high_cardinality_for_this_tag" => {
            return client.should_enable_high_cardinality_for_this_tag(metric_name);
        }
        _ => {}
    }
    None
}
