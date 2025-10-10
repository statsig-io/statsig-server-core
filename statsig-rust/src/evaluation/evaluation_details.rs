use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::SpecsSource;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct EvaluationDetails {
    pub reason: String,
    pub lcut: Option<u64>,
    pub received_at: Option<u64>,
}

impl EvaluationDetails {
    pub fn unrecognized(source: &SpecsSource, lcut: u64, received_at: Option<u64>) -> Self {
        Self::create_from_data(
            source,
            lcut,
            received_at,
            "Unrecognized",
            &EvaluatorResult::default(),
        )
    }

    pub fn recognized_without_eval_result(
        source: &SpecsSource,
        lcut: u64,
        received_at: Option<u64>,
    ) -> Self {
        Self::create_from_data(
            source,
            lcut,
            received_at,
            "Recognized",
            &EvaluatorResult::default(),
        )
    }

    pub fn recognized(
        source: &SpecsSource,
        lcut: u64,
        received_at: Option<u64>,
        eval_result: &EvaluatorResult,
    ) -> Self {
        Self::create_from_data(source, lcut, received_at, "Recognized", eval_result)
    }

    pub fn recognized_but_overridden(
        lcut: u64,
        received_at: Option<u64>,
        override_reason: &str,
    ) -> Self {
        Self {
            reason: format!("{override_reason}:Recognized"),
            lcut: Some(lcut),
            received_at,
        }
    }

    #[must_use]
    pub fn unrecognized_no_data() -> Self {
        Self {
            reason: SpecsSource::NoValues.to_string(),
            lcut: None,
            received_at: None,
        }
    }

    #[must_use]
    pub fn error(sub_reason: &str) -> Self {
        Self {
            reason: format!("Error:{sub_reason}"),
            lcut: None,
            received_at: None,
        }
    }

    fn create_from_data(
        source: &SpecsSource,
        lcut: u64,
        received_at: Option<u64>,
        sub_reason: &str,
        eval_result: &EvaluatorResult,
    ) -> Self {
        if *source == SpecsSource::Uninitialized || *source == SpecsSource::NoValues {
            return Self {
                reason: source.to_string(),
                lcut: None,
                received_at: None,
            };
        }

        if eval_result.unsupported {
            return Self {
                reason: format!("{source}:Unsupported"),
                lcut: Some(lcut),
                received_at,
            };
        }

        Self {
            reason: format!("{}:{}", source, sub_reason),
            lcut: Some(lcut),
            received_at,
        }
    }
}
