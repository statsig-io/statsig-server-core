use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::{
    interned_string::InternedString,
    networking::ResponseData,
    observability::{
        ops_stats::{OpsStatsEvent, OpsStatsForInstance},
        ErrorBoundaryEvent,
    },
    specs_response::{proto_specs::deserialize_protobuf, spec_types::SpecsResponseFull},
    OpsStatsEventObserver, StatsigErr, StatsigRuntime,
};

lazy_static::lazy_static! {
    static ref OPS_STATS: OpsStatsForInstance = OpsStatsForInstance::new();
}

// Generated from cli/src/commands/make-test-proto.ts
// run `tore make-test-proto` to generate the unknown_enum.pb.br file
const UNKNOWN_ENUM_PROTO_BYTES: &[u8] = include_bytes!("../../../tests/data/unknown_enum.pb.br");

struct TestErrorObserver {
    received_events: Mutex<Vec<ErrorBoundaryEvent>>,
}

impl TestErrorObserver {
    fn new() -> Self {
        Self {
            received_events: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl OpsStatsEventObserver for TestErrorObserver {
    async fn handle_event(&self, event: OpsStatsEvent) {
        if let OpsStatsEvent::SDKError(error) = event {
            self.received_events.lock().push(error);
        }
    }
}

#[tokio::test]
async fn test_it_can_parse_response_with_unknown_fields() {
    let result = get_deserialized_spec_result();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_logging_error_to_statsig_and_console() {
    let runtime = StatsigRuntime::get_runtime();
    let _ = runtime
        .spawn("keep_alive", |_| async { /* noop */ })
        .unwrap();

    let observer = Arc::new(TestErrorObserver::new());
    let weak_observer = Arc::downgrade(&observer);
    OPS_STATS.subscribe(runtime.clone(), weak_observer);

    let _ = get_deserialized_spec_result();

    runtime.await_tasks_with_tag("keep_alive").await;

    let received_events = observer.received_events.lock().clone();
    assert_eq!(received_events[0].exception, "ProtobufParseError");
}

#[test]
fn test_it_parsed_the_gate_without_unexpected_field() {
    let result = get_deserialized_spec_result().expect("Failed to deserialize specs");

    let supported_gate = result
        .feature_gates
        .get(&InternedString::from_str_ref(
            "gate_without_unexpected_field",
        ))
        .expect("Gate not found")
        .as_spec_ref();

    assert_eq!(supported_gate.default_value.get_bool(), Some(true));
    assert_eq!(supported_gate.entity, "feature_gate");
    assert_eq!(supported_gate.id_type, "userID");
    assert_eq!(*supported_gate.rules, Vec::new());
    assert_eq!(supported_gate.version, Some(8));
    assert_eq!(
        supported_gate.salt,
        "test_salt_for_gate_without_unexpected_field"
    );
}

#[test]
fn test_it_ignored_the_gate_with_unexpected_field() {
    let result = get_deserialized_spec_result().expect("Failed to deserialize specs");

    let count = result.feature_gates.len();
    assert_eq!(count, 1);
}

#[test]
fn test_it_extracted_known_top_level_fields() {
    let result = get_deserialized_spec_result().expect("Failed to deserialize specs");

    assert_eq!(result.company_id, Some("test_company_id".to_string()));
    assert_eq!(
        result.response_format,
        Some("test_response_format".to_string())
    );
    assert_eq!(result.checksum, Some("test_checksum".to_string()));
}

fn get_deserialized_spec_result() -> Result<SpecsResponseFull, StatsigErr> {
    let left = SpecsResponseFull::default();
    let mut right = SpecsResponseFull::default();

    let mut data = ResponseData::from_bytes(UNKNOWN_ENUM_PROTO_BYTES.to_vec());

    deserialize_protobuf(&OPS_STATS, &left, &mut right, &mut data)?;
    Ok(right)
}
