use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::hashing::HashUtil;
use crate::id_lists_adapter::IdList;
use crate::interned_string::InternedString;
use crate::specs_response::spec_types::{Rule, Spec, SpecsResponseFull};
use crate::user::StatsigUserInternal;
use crate::StatsigErr::StackOverflowError;
use crate::{OverrideAdapter, SecondaryExposure, Statsig, StatsigErr};

const MAX_RECURSIVE_DEPTH: u16 = 300;
// Isolated nested evaluation still consumes native stack at each dependency
// level. Keep its limit below the legacy general limit so malformed cycles
// return an error before exhausting the thread stack.
const MAX_ISOLATED_RECURSIVE_DEPTH: u16 = 64;

// (gate_name, (bool_value, rule_id, secondary_exposures, id_list_source))
type NestedGateMemo = HashMap<
    InternedString,
    (
        bool,
        Option<InternedString>,
        Vec<SecondaryExposure>,
        Option<&'static str>,
    ),
>;

pub(crate) struct ExperimentGroupResolutionGuard {
    path: Rc<RefCell<HashSet<InternedString>>>,
    experiment_name: InternedString,
}

impl Drop for ExperimentGroupResolutionGuard {
    fn drop(&mut self) {
        self.path.borrow_mut().remove(&self.experiment_name);
    }
}

#[derive(Clone, Copy)]
pub enum IdListResolution<'a> {
    MapLookup(&'a HashMap<String, IdList>),
    Callback(&'a dyn Fn(&str, &str) -> bool),
}

pub struct EvaluatorContext<'a> {
    pub user: &'a StatsigUserInternal<'a, 'a>,
    pub specs_data: &'a SpecsResponseFull,
    pub id_list_resolver: IdListResolution<'a>,
    pub hashing: &'a HashUtil,
    pub result: EvaluatorResult,
    pub nested_count: u16,
    // This tracks only the active `evaluate_with_isolated_result` call stack.
    // Unlike `nested_count`, it must survive `reset_result()` because config
    // mapping intentionally resets the latter while evaluating a nested spec.
    isolated_evaluation_depth: u16,
    // Experiment-group references are resolved recursively. This is an active
    // path, rather than a global cache, so sequential references remain valid
    // while a re-entry on the current path safely has no resolvable group.
    experiment_group_resolution_path: Rc<RefCell<HashSet<InternedString>>>,
    pub app_id: Option<&'a DynamicValue>,
    pub override_adapter: Option<&'a Arc<dyn OverrideAdapter>>,
    pub nested_gate_memo: NestedGateMemo,
    pub should_user_third_party_parser: bool,
    pub statsig: Option<&'a Statsig>,
    pub disable_exposure_logging: bool,
    pub gcir_hashes: Vec<u64>,
    // When set, a top-level evaluation only considers the corresponding subset of
    // rules (targeting-only / override-only). Used by persistent-assignment
    // enforceTargeting / enforceOverrides to decide whether a sticky value still holds.
    pub only_evaluate_targeting: bool,
    pub only_evaluate_overrides: bool,
}

impl<'a> EvaluatorContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user: &'a StatsigUserInternal,
        specs_data: &'a SpecsResponseFull,
        id_list_resolver: IdListResolution<'a>,
        hashing: &'a HashUtil,
        app_id: Option<&'a DynamicValue>,
        override_adapter: Option<&'a Arc<dyn OverrideAdapter>>,
        should_user_third_party_parser: bool,
        statsig: Option<&'a Statsig>,
        disable_exposure_logging: bool,
    ) -> Self {
        let result = EvaluatorResult::default();

        Self {
            user,
            specs_data,
            id_list_resolver,
            hashing,
            app_id,
            result,
            override_adapter,
            nested_count: 0,
            isolated_evaluation_depth: 0,
            experiment_group_resolution_path: Rc::new(RefCell::new(HashSet::new())),
            nested_gate_memo: HashMap::new(),
            should_user_third_party_parser,
            statsig,
            disable_exposure_logging,
            gcir_hashes: Vec::new(),
            only_evaluate_targeting: false,
            only_evaluate_overrides: false,
        }
    }

    pub fn reset_result(&mut self) {
        self.nested_count = 0;
        self.result = EvaluatorResult::default();
    }

    /// Enters a path-local experiment-group resolution scope. The returned
    /// guard removes the name during normal returns, errors, and panic unwind.
    pub(crate) fn enter_experiment_group_resolution(
        &self,
        experiment_name: InternedString,
    ) -> Option<ExperimentGroupResolutionGuard> {
        if !self
            .experiment_group_resolution_path
            .borrow_mut()
            .insert(experiment_name.clone())
        {
            return None;
        }

        Some(ExperimentGroupResolutionGuard {
            path: Rc::clone(&self.experiment_group_resolution_path),
            experiment_name,
        })
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

    /// Evaluates a nested spec with an isolated result while preserving all
    /// other context state, including the shared nested-gate memo.
    ///
    /// The parent result and recursive depths are restored on both success and
    /// error. Callers explicitly merge only the child fields owned by their
    /// condition, preventing nested results from replacing the outer result.
    pub(crate) fn evaluate_with_isolated_result<T>(
        &mut self,
        evaluate: impl FnOnce(&mut Self) -> Result<T, StatsigErr>,
    ) -> Result<(T, EvaluatorResult), StatsigErr> {
        if self.isolated_evaluation_depth >= MAX_ISOLATED_RECURSIVE_DEPTH {
            return Err(StackOverflowError);
        }

        let parent_result = std::mem::take(&mut self.result);
        let parent_nested_count = self.nested_count;
        let parent_isolated_evaluation_depth = self.isolated_evaluation_depth;
        self.nested_count += 1;
        self.isolated_evaluation_depth += 1;

        let evaluation = evaluate(self);

        self.nested_count = parent_nested_count;
        self.isolated_evaluation_depth = parent_isolated_evaluation_depth;
        let child_result = std::mem::replace(&mut self.result, parent_result);

        evaluation.map(|value| (value, child_result))
    }
}
