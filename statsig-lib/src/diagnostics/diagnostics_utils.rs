use std::collections::HashMap;
use super::{diagnostics::ContextType, marker::Marker};

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
            Err(err) => {
                Err(format!("Failed to serialize markers: {}", err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{marker::{StepType, ActionType, KeyType, Marker}, diagnostics::ContextType};

    #[test]
    fn test_format_diagnostics_metadata_with_valid_markers() {
        let start_marker = Marker::new(KeyType::Initialize, ActionType::Start, Some(StepType::Process))
            .with_is_success(true);
        let end_marker = Marker::new(KeyType::Initialize, ActionType::End, Some(StepType::Process))
            .with_status_code(200);

        let markers = vec![start_marker, end_marker];
        let context_type = ContextType::Initialize;
        let metadata = DiagnosticsUtils::format_diagnostics_metadata(&context_type, &markers)
            .expect("format_diagnostics_metadata returned an Err");
        let markers_json = metadata.get("markers").expect("Markers not found in metadata");
        let parsed_markers: serde_json::Value =
            serde_json::from_str(markers_json).expect("Failed to parse markers JSON into serde_json::Value");
        
        let expected_json = serde_json::json!([
            {
                "key": "initialize",
                "action": "start",
                "step": "process",
                "success": true,
                "timestamp": null,
                "statusCode": null,
                "url": null,
                "idListCount": null,
                "sdkRegion": null,
                "markerID": null,
                "attempt": null,
                "configName": null,
                "message": null,
                "error": null
            },
            {
                "key": "initialize",
                "action": "end",
                "step": "process",
                "success": null,
                "timestamp": null,
                "statusCode": 200,
                "url": null,
                "idListCount": null,
                "sdkRegion": null,
                "markerID": null,
                "attempt": null,
                "configName": null,
                "message": null,
                "error": null
            }
        ]);
        
        assert_eq!(metadata.get("context").expect("Context not found in metadata"), "initialize");
        assert_eq!(parsed_markers, expected_json);
    }
}