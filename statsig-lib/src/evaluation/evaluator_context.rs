use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::memo_sha_256::MemoSha256;
use crate::spec_store::SpecStoreData;
use crate::statsig_user_internal::StatsigUserInternal;

pub struct EvaluatorContext<'a> {
    pub user: &'a StatsigUserInternal,
    pub spec_store_data: &'a SpecStoreData,
    pub sha_hasher: &'a MemoSha256,
    pub result: EvaluatorResult<'a>,
    pub nested_count: u64,
}

impl<'a> EvaluatorContext<'a> {
    pub fn new(
        user: &'a StatsigUserInternal,
        spec_store_data: &'a SpecStoreData,
        sha_hasher: &'a MemoSha256,
    ) -> Self {
        let result = EvaluatorResult::default();

        Self {
            user,
            spec_store_data,
            sha_hasher,
            result,
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

    pub fn increment_nesting(&mut self) {
        self.nested_count += 1;
    }
}
