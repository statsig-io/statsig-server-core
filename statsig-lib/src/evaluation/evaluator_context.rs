use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::memo_sha_256::MemoSha256;
use crate::spec_store::SpecStoreData;
use crate::statsig_user_internal::StatsigUserInternal;

pub struct EvaluatorContext<'a> {
    pub user: &'a StatsigUserInternal,
    pub spec_store_data: &'a SpecStoreData,
    pub sha_hasher: &'a MemoSha256,
    pub result: EvaluatorResult<'a>,
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
        }
    }

    pub fn reset_result(&mut self) {
        self.result = EvaluatorResult::default()
    }
}
