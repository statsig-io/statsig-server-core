use crate::spec_store::SpecStoreData;
use serde::{Deserialize, Serialize};
use crate::SpecsSource;

#[derive(Serialize, Deserialize, Clone)]
pub struct EvaluationDetails {
    pub reason: String,
    pub lcut: Option<u64>,
    pub received_at: Option<u64>,
}

impl EvaluationDetails {
    pub fn unrecognized(spec_store_data: &SpecStoreData) -> Self {
        Self::create_from_data(spec_store_data, "Unrecognized")
    }

    pub fn recognized(spec_store_data: &SpecStoreData) -> Self {
        Self::create_from_data(spec_store_data, "Recognized")
    }

    pub fn unrecognized_no_data() -> Self {
        Self {
            reason: SpecsSource::NoValues.to_string(),
            lcut: None,
            received_at: None,
        }
    }

    pub fn error(sub_reason: &str) -> Self {
        Self {
            reason: format!("Error:{}", sub_reason),
            lcut: None,
            received_at: None,
        }
    }

    fn create_from_data(data: &SpecStoreData, sub_reason: &str) -> Self {
        if data.source == SpecsSource::Uninitialized || data.source == SpecsSource::NoValues {
            return Self {
                reason: data.source.to_string(),
                lcut: None,
                received_at: None,
            };
        }

        Self {
            reason: format!("{}:{}", data.source, sub_reason),
            lcut: Some(data.values.time),
            received_at: data.time_received_at,
        }
    }
}
