use super::{diagnostics::ContextType, marker::Marker};
use std::collections::HashMap;

pub struct DiagnosticsUtils;

impl DiagnosticsUtils {
    pub fn format_diagnostics_metadata(
        context_type: &ContextType,
        markers: &Vec<Marker>,
    ) -> Result<HashMap<String, String>, String> {
        let mut metadata = HashMap::new();

        metadata.insert("context".to_string(), context_type.to_string());

        match serde_json::to_string(&markers) {
            Ok(markers_json) => {
                metadata.insert("markers".to_string(), markers_json);
                Ok(metadata)
            }
            Err(err) => Err(format!("Failed to serialize markers: {}", err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{
        diagnostics::ContextType,
        marker::{ActionType, KeyType, Marker, StepType},
    };

    #[test]
    fn test_format_diagnostics_metadata_with_valid_markers() {
        let start_timestamp: u64 = 1_640_995_200_000; // 2022-01-01T00:00:00Z
        let end_timestamp: u64 = 1_640_995_800_000; // 10 minutes later

        let start_marker = Marker::new(
            KeyType::Initialize,
            ActionType::Start,
            Some(StepType::Process),
            start_timestamp,
        )
        .with_is_success(true);

        let end_marker = Marker::new(
            KeyType::Initialize,
            ActionType::End,
            Some(StepType::Process),
            end_timestamp,
        )
        .with_status_code(200);

        let markers = vec![start_marker, end_marker];
        let context_type = ContextType::Initialize;
        let metadata = DiagnosticsUtils::format_diagnostics_metadata(&context_type, &markers)
            .expect("format_diagnostics_metadata returned an Err");
        let markers_json = metadata
            .get("markers")
            .expect("Markers not found in metadata");
        let parsed_markers: serde_json::Value = serde_json::from_str(markers_json)
            .expect("Failed to parse markers JSON into serde_json::Value");

        let expected_json = serde_json::json!([
            {
                "key": "initialize",
                "action": "start",
                "step": "process",
                "success": true,
                "timestamp": start_timestamp,
            },
            {
                "key": "initialize",
                "action": "end",
                "step": "process",
                "timestamp": end_timestamp,
                "statusCode": 200,
            }
        ]);

        assert_eq!(
            metadata
                .get("context")
                .expect("Context not found in metadata"),
            "initialize"
        );
        assert_eq!(parsed_markers, expected_json);
    }
}
