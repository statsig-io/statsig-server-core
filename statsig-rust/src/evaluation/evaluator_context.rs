use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::hashing::HashUtil;
use crate::spec_store::SpecStoreData;
use crate::spec_types::{Rule, Spec};
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
}

impl<'a> EvaluatorContext<'a> {
    pub fn new(
        user: &'a StatsigUserInternal,
        spec_store_data: &'a SpecStoreData,
        hashing: &'a HashUtil,
        app_id: &'a Option<&'a DynamicValue>,
        override_adapter: &'a Option<Arc<dyn OverrideAdapter>>,
    ) -> Self {
        let result = EvaluatorResult::default();

        Self {
            user,
            spec_store_data,
            hashing,
            app_id,
            result,
            override_adapter,
            nested_count: 0,
        }
    }

    pub fn reset_result(&mut self) {
        self.result = EvaluatorResult::default();
    }

    pub fn finalize_evaluation(&mut self, spec: &Spec, rule: Option<&Rule>) {
        self.result.sampling_rate = rule.and_then(|r| r.sampling_rate);
        self.result.forward_all_exposures = spec.forward_all_exposures;

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

        self.result.undelegated_secondary_exposures = Some(self.result.secondary_exposures.clone());
    }

    pub fn prep_for_nested_evaluation(&mut self) -> Result<(), StatsigErr> {
        self.nested_count += 1;

        self.result.bool_value = false;
        self.result.json_value = None;

        if self.nested_count > MAX_RECURSIVE_DEPTH {
            return Err(StackOverflowError);
        }

        Ok(())
    }
}
