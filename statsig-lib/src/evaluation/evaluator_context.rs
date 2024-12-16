use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::hashing::HashUtil;
use crate::spec_store::SpecStoreData;
use crate::statsig_core_api_options::AnyEvaluationOptions;
use crate::statsig_user_internal::StatsigUserInternal;
use crate::StatsigErr::StackOverflowError;
use crate::{OverrideAdapter, StatsigErr};
use std::sync::Arc;

const MAX_RECURSIVE_DEPTH: u16 = 300;

pub struct EvaluatorContext<'a> {
    pub user: &'a StatsigUserInternal,
    pub spec_store_data: &'a SpecStoreData,
    pub hashing: &'a HashUtil,
    pub result: EvaluatorResult<'a>,
    pub nested_count: u16,
    pub app_id: &'a Option<&'a DynamicValue>,
    pub override_adapter: &'a Option<Arc<dyn OverrideAdapter>>,
    pub evaluation_options: &'a Option<AnyEvaluationOptions>,
}

impl<'a> EvaluatorContext<'a> {
    pub fn new(
        user: &'a StatsigUserInternal,
        spec_store_data: &'a SpecStoreData,
        hashing: &'a HashUtil,
        app_id: &'a Option<&'a DynamicValue>,
        override_adapter: &'a Option<Arc<dyn OverrideAdapter>>,
        evaluation_options: &'a Option<AnyEvaluationOptions>,
    ) -> Self {
        let result = EvaluatorResult::default();

        Self {
            user,
            spec_store_data,
            hashing,
            app_id,
            result,
            override_adapter,
            evaluation_options,
            nested_count: 0,
        }
    }

    pub fn reset_result(&mut self) {
        self.result = EvaluatorResult::default()
    }

    pub fn finalize_evaluation(&mut self) {
        if self.nested_count > 0 {
            self.nested_count -= 1;
            return;
        }

        if self.result.secondary_exposures.is_empty() {
            return;
        }

        if self.result.undelegated_secondary_exposures.is_some() {
            return;
        }

        self.result.undelegated_secondary_exposures = Some(self.result.secondary_exposures.clone())
    }

    pub fn increment_nesting(&mut self) -> Result<(), StatsigErr> {
        self.nested_count += 1;

        if self.nested_count > MAX_RECURSIVE_DEPTH {
            return Err(StackOverflowError);
        }

        Ok(())
    }
}
