use std::collections::HashMap;
use std::sync::Arc;

use napi_derive::napi;
use statsig_rust::{ObservabilityClient as ObservabilityClientActual, OpsStatsEventObserver};

use napi::bindgen_prelude::{FnArgs, Promise};
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};

type MetricFnArgs = FnArgs<(String, f64, Option<HashMap<String, String>>)>;
type MetricFn = ThreadsafeFunction<MetricFnArgs, Promise<()>, MetricFnArgs, false>;

#[napi(object, object_to_js = false)]
pub struct ObservabilityClient {
    #[napi(js_name = "initialize", ts_type = "() => void")]
    pub initialize_fn: Option<ThreadsafeFunction<()>>,

    #[napi(
        js_name = "increment",
        ts_type = "(metricName: string, value: number, tags: Record<string, string>) => void"
    )]
    pub increment_fn: Option<MetricFn>,

    #[napi(
        js_name = "gauge",
        ts_type = "(metricName: string, value: number, tags: Record<string, string>) => void"
    )]
    pub gauge_fn: Option<MetricFn>,

    #[napi(
        js_name = "dist",
        ts_type = "(metricName: string, value: number, tags: Record<string, string>) => void"
    )]
    pub dist_fn: Option<MetricFn>,

    #[napi(js_name = "error", ts_type = "(tag: string, error: string) => void")]
    pub error_fn: Option<ThreadsafeFunction<(String, String)>>,
}

impl ObservabilityClientActual for ObservabilityClient {
    fn init(&self) {
        let fnc = match &self.initialize_fn {
            Some(f) => f,
            None => return,
        };

        fnc.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
    }

    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        let fnc = match &self.increment_fn {
            Some(f) => f,
            None => return,
        };

        let args = (metric_name, value, tags).into();
        fnc.call(args, ThreadsafeFunctionCallMode::Blocking);
    }

    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        let fnc = match &self.gauge_fn {
            Some(f) => f,
            None => return,
        };

        let args = (metric_name, value, tags).into();
        fnc.call(args, ThreadsafeFunctionCallMode::Blocking);
    }

    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        let fnc = match &self.dist_fn {
            Some(f) => f,
            None => return,
        };

        let args = (metric_name, value, tags).into();
        fnc.call(args, ThreadsafeFunctionCallMode::Blocking);
    }

    fn error(&self, tag: String, error: String) {
        let fnc = match &self.error_fn {
            Some(f) => f,
            None => return,
        };

        fnc.call(Ok((tag, error)), ThreadsafeFunctionCallMode::Blocking);
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }
}

#[napi(js_name = "__internal__testObservabilityClient")]
pub async fn test_observability_client(
    client: ObservabilityClient,
    action: String,
    metric_name: String,
    value: f64,
    tags: Option<HashMap<String, String>>,
) {
    // This needs to be async. It returns a promise to the JS side to ensure the order of operations.
    match action.as_str() {
        "init" => client.init(),
        "increment" => client.increment(metric_name, value, tags),
        "gauge" => client.gauge(metric_name, value, tags),
        "dist" => client.dist(metric_name, value, tags),
        _ => panic!("Invalid action: {}", action),
    }
}
