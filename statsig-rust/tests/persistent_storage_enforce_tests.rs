mod utils;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::json;
use statsig_rust::{
    ExperimentEvaluationOptions, LayerEvaluationOptions, PersistentStorage, Statsig,
    StatsigOptions, StatsigUser, StatsigUserBuilder, StickyValues, UserPersistedValues,
};
use utils::mock_specs_adapter::MockSpecsAdapter;

const EXPERIMENT: &str = "targeted_exp_in_unlayered_with_holdout";
// Matches the override rule condition (`userID any [...]`) for the experiment above.
const OVERRIDE_USER_ID: &str = "exp_override_targeted_exp_in_unlayered_with_holdout_controll";
const OVERRIDE_RULE_ID: &str = "6AGyymqmwAOlJ2SevKrX80:userID:id_override";
const TEST_GROUP_RULE_ID: &str = "6AGyyo5oYSXQ2sqa6SnAr2";

#[derive(Default)]
struct MockPersistentStorage {
    store: Mutex<HashMap<String, UserPersistedValues>>,
}

impl PersistentStorage for MockPersistentStorage {
    fn load(&self, key: String) -> Option<UserPersistedValues> {
        self.store.lock().unwrap().get(&key).cloned()
    }

    fn save(&self, key: &str, config_name: &str, data: StickyValues) {
        self.store
            .lock()
            .unwrap()
            .entry(key.to_string())
            .or_default()
            .insert(config_name.to_string(), data);
    }

    fn delete(&self, key: &str, config_name: &str) {
        if let Some(values) = self.store.lock().unwrap().get_mut(key) {
            values.remove(config_name);
        }
    }
}

async fn make_statsig() -> (Statsig, Arc<MockPersistentStorage>) {
    let storage = Arc::new(MockPersistentStorage::default());
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            persistent_storage: Some(storage.clone()),
            ..StatsigOptions::new()
        })),
    );
    statsig.initialize().await.unwrap();
    (statsig, storage)
}

// A sticky value that pins the user to the "Test" group, regardless of what the
// live evaluation would return.
fn sticky_test_group() -> UserPersistedValues {
    let sticky: StickyValues = serde_json::from_value(json!({
        "value": true,
        "json_value": { "exp_val": "targeted_test" },
        "rule_id": TEST_GROUP_RULE_ID,
        "group_name": "Test",
        "secondary_exposures": [],
        "time": 1_700_000_000_000i64,
    }))
    .unwrap();

    let mut values = HashMap::new();
    values.insert(EXPERIMENT.to_string(), sticky);
    values
}

fn override_user() -> StatsigUser {
    StatsigUserBuilder::new_with_user_id(OVERRIDE_USER_ID.to_string()).build()
}

#[tokio::test]
async fn test_sticky_value_wins_without_enforce_overrides() {
    let (statsig, _storage) = make_statsig().await;

    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&override_user(), EXPERIMENT, options);
    let _ = statsig.shutdown().await;

    // The user matches the console override rule, but a sticky value exists and
    // enforceOverrides is off, so the persisted value wins (the reported bug).
    assert_eq!(experiment.group_name.as_deref(), Some("Test"));
    assert_eq!(experiment.rule_id, TEST_GROUP_RULE_ID);
    assert_eq!(experiment.value["exp_val"], "targeted_test");
    assert_eq!(experiment.details.reason, "Persisted");
}

#[tokio::test]
async fn test_enforce_overrides_lets_override_win_over_sticky() {
    let (statsig, _storage) = make_statsig().await;

    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        enforce_overrides: true,
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&override_user(), EXPERIMENT, options);
    let _ = statsig.shutdown().await;

    // With enforceOverrides, the matching override rule takes precedence over the
    // persisted sticky value.
    assert_eq!(experiment.group_name.as_deref(), Some("Control"));
    assert_eq!(experiment.rule_id, OVERRIDE_RULE_ID);
    assert_eq!(experiment.value["exp_val"], "control");
    assert_ne!(experiment.details.reason, "Persisted");
}

#[tokio::test]
async fn test_enforce_overrides_keeps_sticky_when_no_override_matches() {
    let (statsig, _storage) = make_statsig().await;

    // This user does not match the override rule.
    let user = StatsigUserBuilder::new_with_user_id("not-overridden-user".to_string()).build();
    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        enforce_overrides: true,
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&user, EXPERIMENT, options);
    let _ = statsig.shutdown().await;

    // No override rule matches, so the sticky value is honored even with
    // enforceOverrides on.
    assert_eq!(experiment.group_name.as_deref(), Some("Test"));
    assert_eq!(experiment.details.reason, "Persisted");
}

#[tokio::test]
async fn test_enforce_targeting_drops_sticky_when_no_longer_targeted() {
    let (statsig, _storage) = make_statsig().await;

    // Find a user that is no longer targeted by the experiment. `targetingGate`
    // is a fail_gate on `test_50_50`, so a user that fails `test_50_50` is not
    // targeted and their sticky value should be dropped under enforceTargeting.
    let mut untargeted_user: Option<StatsigUser> = None;
    for i in 0..200 {
        let candidate = StatsigUserBuilder::new_with_user_id(format!("targeting-user-{i}")).build();
        if !statsig.check_gate(&candidate, "test_50_50") {
            untargeted_user = Some(candidate);
            break;
        }
    }
    let user = untargeted_user.expect("expected to find a user that fails test_50_50");

    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        enforce_targeting: true,
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&user, EXPERIMENT, options);
    let _ = statsig.shutdown().await;

    // The user no longer passes targeting, so the persisted value is dropped in
    // favor of the live evaluation.
    assert_ne!(experiment.details.reason, "Persisted");
}

#[tokio::test]
async fn test_enforce_targeting_keeps_sticky_when_still_targeted() {
    let (statsig, _storage) = make_statsig().await;

    // A user that passes `test_50_50` is still targeted by the experiment.
    let mut targeted_user: Option<StatsigUser> = None;
    for i in 0..200 {
        let candidate = StatsigUserBuilder::new_with_user_id(format!("targeting-user-{i}")).build();
        if statsig.check_gate(&candidate, "test_50_50") {
            targeted_user = Some(candidate);
            break;
        }
    }
    let user = targeted_user.expect("expected to find a user that passes test_50_50");

    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        enforce_targeting: true,
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&user, EXPERIMENT, options);
    let _ = statsig.shutdown().await;

    // Still targeted -> the sticky value is honored.
    assert_eq!(experiment.group_name.as_deref(), Some("Test"));
    assert_eq!(experiment.details.reason, "Persisted");
}

// -------------------------------------------------------------------- [ Layers ]

const LAYER: &str = "layer_with_many_params";
// The layer's allocated experiment; its override rule matches `user-in-control`.
const LAYER_DELEGATE: &str = "experiment_with_many_params";
const LAYER_OVERRIDE_USER_ID: &str = "user-in-control";

// A sticky layer value pinned to the "Test" group of the allocated experiment.
fn sticky_layer_test_group() -> UserPersistedValues {
    let sticky: StickyValues = serde_json::from_value(json!({
        "value": true,
        "json_value": { "a_string": "sticky_test" },
        "rule_id": "7kGqFaUIGHjHJ5X7SOKJcM",
        "group_name": "Test",
        "secondary_exposures": [],
        "config_delegate": LAYER_DELEGATE,
        "time": 1_700_000_000_000i64,
    }))
    .unwrap();

    let mut values = HashMap::new();
    values.insert(LAYER.to_string(), sticky);
    values
}

#[tokio::test]
async fn test_layer_sticky_value_wins_without_enforce_overrides() {
    let (statsig, _storage) = make_statsig().await;

    let user = StatsigUserBuilder::new_with_user_id(LAYER_OVERRIDE_USER_ID.to_string()).build();
    let options = LayerEvaluationOptions {
        user_persisted_values: Some(sticky_layer_test_group()),
        ..Default::default()
    };
    let layer = statsig.get_layer_with_options(&user, LAYER, options);
    let _ = statsig.shutdown().await;

    assert_eq!(layer.group_name.as_deref(), Some("Test"));
    assert_eq!(layer.details.reason, "Persisted");
}

#[tokio::test]
async fn test_layer_enforce_overrides_lets_override_win_over_sticky() {
    let (statsig, _storage) = make_statsig().await;

    let user = StatsigUserBuilder::new_with_user_id(LAYER_OVERRIDE_USER_ID.to_string()).build();
    let options = LayerEvaluationOptions {
        user_persisted_values: Some(sticky_layer_test_group()),
        enforce_overrides: true,
        ..Default::default()
    };
    let layer = statsig.get_layer_with_options(&user, LAYER, options);
    let _ = statsig.shutdown().await;

    // The allocated experiment's override rule matches, so the live evaluation
    // wins over the persisted value.
    assert_ne!(layer.details.reason, "Persisted");
    assert_eq!(layer.__value["a_string"], "control");
}

#[tokio::test]
async fn test_layer_enforce_targeting_drops_sticky_when_no_longer_targeted() {
    let (statsig, _storage) = make_statsig().await;

    // This layer's allocated experiment has `targetingGate` = fail_gate on
    // `test_50_50`, so a user failing that gate is no longer targeted.
    let mut untargeted_user: Option<StatsigUser> = None;
    for i in 0..200 {
        let candidate = StatsigUserBuilder::new_with_user_id(format!("targeting-user-{i}")).build();
        if !statsig.check_gate(&candidate, "test_50_50") {
            untargeted_user = Some(candidate);
            break;
        }
    }
    let user = untargeted_user.expect("expected to find a user that fails test_50_50");

    let sticky: StickyValues = serde_json::from_value(json!({
        "value": true,
        "json_value": {},
        "rule_id": "6h3c5rGSyHu4p1sbIBqltg",
        "group_name": "Test",
        "secondary_exposures": [],
        "config_delegate": "targeted_exp_in_layer_with_holdout",
        "time": 1_700_000_000_000i64,
    }))
    .unwrap();
    let mut values = HashMap::new();
    values.insert("test_layer_in_holdout".to_string(), sticky);

    let options = LayerEvaluationOptions {
        user_persisted_values: Some(values),
        enforce_targeting: true,
        ..Default::default()
    };
    let layer = statsig.get_layer_with_options(&user, "test_layer_in_holdout", options);
    let _ = statsig.shutdown().await;

    // The user no longer passes the allocated experiment's targeting, so the
    // persisted value is dropped in favor of the live evaluation.
    assert_ne!(layer.details.reason, "Persisted");
}

#[tokio::test]
async fn test_layer_enforce_targeting_keeps_sticky_when_still_targeted() {
    let (statsig, _storage) = make_statsig().await;

    let mut targeted_user: Option<StatsigUser> = None;
    for i in 0..200 {
        let candidate = StatsigUserBuilder::new_with_user_id(format!("targeting-user-{i}")).build();
        if statsig.check_gate(&candidate, "test_50_50") {
            targeted_user = Some(candidate);
            break;
        }
    }
    let user = targeted_user.expect("expected to find a user that passes test_50_50");

    let sticky: StickyValues = serde_json::from_value(json!({
        "value": true,
        "json_value": {},
        "rule_id": "6h3c5rGSyHu4p1sbIBqltg",
        "group_name": "Test",
        "secondary_exposures": [],
        "config_delegate": "targeted_exp_in_layer_with_holdout",
        "time": 1_700_000_000_000i64,
    }))
    .unwrap();
    let mut values = HashMap::new();
    values.insert("test_layer_in_holdout".to_string(), sticky);

    let options = LayerEvaluationOptions {
        user_persisted_values: Some(values),
        enforce_targeting: true,
        ..Default::default()
    };
    let layer = statsig.get_layer_with_options(&user, "test_layer_in_holdout", options);
    let _ = statsig.shutdown().await;

    // Still targeted -> the sticky value is honored.
    assert_eq!(layer.group_name.as_deref(), Some("Test"));
    assert_eq!(layer.details.reason, "Persisted");
}

// -------------------------------------------------------------------- [ Edge Cases ]

#[tokio::test]
async fn test_no_sticky_value_returns_live_evaluation_with_enforce_flags() {
    let (statsig, _storage) = make_statsig().await;

    // No persisted value for this experiment: enforce flags must be a no-op and
    // return the live evaluation without panicking.
    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(HashMap::new()),
        enforce_overrides: true,
        enforce_targeting: true,
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&override_user(), EXPERIMENT, options);
    let _ = statsig.shutdown().await;

    // Live evaluation for the override user returns the override group.
    assert_eq!(experiment.group_name.as_deref(), Some("Control"));
    assert_eq!(experiment.rule_id, OVERRIDE_RULE_ID);
    assert_ne!(experiment.details.reason, "Persisted");
}

#[tokio::test]
async fn test_enforce_overrides_without_override_rules_keeps_sticky() {
    let (statsig, _storage) = make_statsig().await;

    // `test_experiment_no_targeting` has no override rules: the override-only
    // re-evaluation finds an empty rule subset, which means "not overridden",
    // so the sticky value is honored (mirrors the legacy empty-subset comment).
    let sticky: StickyValues = serde_json::from_value(json!({
        "value": true,
        "json_value": { "value": "sticky" },
        "rule_id": "54QJzvjSk47erparymTMJ6",
        "group_name": "Test",
        "secondary_exposures": [],
        "time": 1_700_000_000_000i64,
    }))
    .unwrap();
    let mut values = HashMap::new();
    values.insert("test_experiment_no_targeting".to_string(), sticky);

    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(values),
        enforce_overrides: true,
        ..Default::default()
    };
    let user = StatsigUserBuilder::new_with_user_id("any-user".to_string()).build();
    let experiment =
        statsig.get_experiment_with_options(&user, "test_experiment_no_targeting", options);
    let _ = statsig.shutdown().await;

    assert_eq!(experiment.group_name.as_deref(), Some("Test"));
    assert_eq!(experiment.details.reason, "Persisted");
}

#[tokio::test]
async fn test_enforce_targeting_without_targeting_rules_keeps_sticky() {
    let (statsig, _storage) = make_statsig().await;

    // No targeting rules on the spec: the targeting-only re-evaluation finds an
    // empty subset, which means the user passes targeting, so sticky is honored.
    let sticky: StickyValues = serde_json::from_value(json!({
        "value": true,
        "json_value": { "value": "sticky" },
        "rule_id": "54QJzvjSk47erparymTMJ6",
        "group_name": "Test",
        "secondary_exposures": [],
        "time": 1_700_000_000_000i64,
    }))
    .unwrap();
    let mut values = HashMap::new();
    values.insert("test_experiment_no_targeting".to_string(), sticky);

    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(values),
        enforce_targeting: true,
        ..Default::default()
    };
    let user = StatsigUserBuilder::new_with_user_id("any-user".to_string()).build();
    let experiment =
        statsig.get_experiment_with_options(&user, "test_experiment_no_targeting", options);
    let _ = statsig.shutdown().await;

    assert_eq!(experiment.group_name.as_deref(), Some("Test"));
    assert_eq!(experiment.details.reason, "Persisted");
}

#[tokio::test]
async fn test_enforce_flags_on_inactive_experiment_deletes_sticky() {
    let (statsig, storage) = make_statsig().await;

    let user = StatsigUserBuilder::new_with_user_id("inactive-user".to_string()).build();
    let storage_key = "inactive-user:userID";
    let experiment_name = "not_started_exp_unlayered_no_holdout"; // isActive: false

    // Pre-populate storage so the delete side effect is observable.
    let sticky: StickyValues = serde_json::from_value(json!({
        "value": true,
        "json_value": { "value": "sticky" },
        "rule_id": "some_rule",
        "group_name": "Test",
        "secondary_exposures": [],
        "time": 1_700_000_000_000i64,
    }))
    .unwrap();
    storage.save(storage_key, experiment_name, sticky.clone());

    let mut values = HashMap::new();
    values.insert(experiment_name.to_string(), sticky);
    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(values),
        enforce_overrides: true,
        enforce_targeting: true,
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&user, experiment_name, options);
    let _ = statsig.shutdown().await;

    // Inactive experiment: pre-existing behavior (sticky deleted, live returned)
    // is unchanged by the enforce flags, which are checked only after the
    // is-active early exit.
    assert_ne!(experiment.details.reason, "Persisted");
    let store = storage.store.lock().unwrap();
    assert!(!store
        .get(storage_key)
        .is_some_and(|v| v.contains_key(experiment_name)));
}

// The raw paths are what the FFI bindings (e.g. Java via JNI) call.
// Run with: cargo test --features ffi-support
#[cfg(feature = "ffi-support")]
#[tokio::test]
async fn test_raw_experiment_enforce_overrides_lets_override_win_over_sticky() {
    let (statsig, _storage) = make_statsig().await;

    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        enforce_overrides: true,
        ..Default::default()
    };
    let (group_name, reason) =
        statsig.use_raw_experiment_with_options(&override_user(), EXPERIMENT, options, |raw| {
            (
                raw.group_name.map(|g| g.unperformant_to_string()),
                raw.details.reason.clone(),
            )
        });

    let options_no_enforce = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        ..Default::default()
    };
    let (sticky_group, sticky_reason) = statsig.use_raw_experiment_with_options(
        &override_user(),
        EXPERIMENT,
        options_no_enforce,
        |raw| {
            (
                raw.group_name.map(|g| g.unperformant_to_string()),
                raw.details.reason.clone(),
            )
        },
    );
    let _ = statsig.shutdown().await;

    assert_eq!(group_name.as_deref(), Some("Control"));
    assert_ne!(reason, "Persisted");
    assert_eq!(sticky_group.as_deref(), Some("Test"));
    assert_eq!(sticky_reason, "Persisted");
}

#[tokio::test]
async fn test_enforcement_drop_does_not_modify_storage() {
    let (statsig, storage) = make_statsig().await;

    // Override applies and enforcement drops the sticky value. Matching the
    // legacy SDK, the stored sticky value is neither deleted nor overwritten
    // with the live result.
    let options = ExperimentEvaluationOptions {
        user_persisted_values: Some(sticky_test_group()),
        enforce_overrides: true,
        ..Default::default()
    };
    let experiment = statsig.get_experiment_with_options(&override_user(), EXPERIMENT, options);
    let _ = statsig.shutdown().await;

    assert_eq!(experiment.group_name.as_deref(), Some("Control"));
    let store = storage.store.lock().unwrap();
    assert!(store.is_empty(), "no save/delete should have occurred");
}
