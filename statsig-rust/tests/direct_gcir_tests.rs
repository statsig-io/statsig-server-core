mod utils;

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::future::join_all;
use statsig_rust::{
    evaluation::evaluator_context::{EvaluatorContext, IdListResolution},
    gcir::gcir_formatter::GCIRFormatter,
    hashing::HashUtil,
    specs_response::spec_types::SpecsResponseFull,
    user::StatsigUserInternal,
    ClientInitResponseOptions, HashAlgorithm, InitializeResponse, StatsigUser,
};
use tokio::time::Instant;

use crate::utils::helpers::load_contents;

#[test]
fn test_gcir() {
    let specs_data = load_specs_data("eval_proj_dcs.json");

    let id_lists = HashMap::new();
    let response = generate_response(&specs_data, IdListResolution::MapLookup(&id_lists));

    let gate = response
        .feature_gates
        .get("test_public")
        .expect("should have a gate");

    assert!(gate.value);
}

#[test]
fn test_id_list_resolution_via_callback() {
    let specs_data = load_specs_data("eval_proj_dcs.json");

    let id_list: HashMap<String, HashSet<String>> = HashMap::from([(
        "company_id_list".to_string(),
        HashSet::from(["pmWkWSBC".to_string()]),
    )]);

    let id_lists_callback = |list_name: &str, lookup_id: &str| match id_list.get(list_name) {
        Some(list) => list.contains(lookup_id),
        None => false,
    };

    let response = generate_response(&specs_data, IdListResolution::Callback(&id_lists_callback));
    let gate = response
        .feature_gates
        .get("test_id_list")
        .expect("should have a gate");

    assert!(gate.value);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_different_specs_payloads() {
    let id_lists = Arc::new(HashMap::new());

    let eval_proj_dcs = Arc::new(load_specs_data("eval_proj_dcs.json"));
    let big_number_dcs = Arc::new(load_specs_data("big_number_dcs.json"));

    let mut tasks = Vec::new();
    let success_count = Arc::new(AtomicUsize::new(0));

    let deadline = Instant::now() + Duration::from_millis(10);

    for i in 0..100 {
        let eval_proj_dcs = eval_proj_dcs.clone();
        let big_number_dcs = big_number_dcs.clone();
        let id_lists = id_lists.clone();
        let success_count = success_count.clone();

        let task = tokio::spawn(async move {
            // ensure all tasks are queued up before we start generating responses
            tokio::time::sleep_until(deadline).await;

            if i % 2 == 0 {
                let res = generate_response(&eval_proj_dcs, IdListResolution::MapLookup(&id_lists));
                if res.dynamic_configs.len() == 62 {
                    success_count.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                let res =
                    generate_response(&big_number_dcs, IdListResolution::MapLookup(&id_lists));
                if res.dynamic_configs.len() == 1 {
                    success_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        tasks.push(task);
    }

    join_all(tasks).await;

    assert_eq!(success_count.load(Ordering::SeqCst), 100);
}

fn load_specs_data(path: &str) -> SpecsResponseFull {
    let contents = load_contents(path);
    serde_json::from_str::<SpecsResponseFull>(&contents).expect("should parse specs data")
}

fn generate_response(
    specs_data: &SpecsResponseFull,
    id_list_resolution: IdListResolution,
) -> InitializeResponse {
    let mut user = StatsigUser::with_user_id("dan_1");
    user.set_custom_ids(HashMap::from([(
        "companyID".to_string(),
        "123".to_string(),
    )]));
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let hashing = HashUtil::new();

    let mut ctx = EvaluatorContext::new(
        &user_internal,
        specs_data,
        id_list_resolution,
        &hashing,
        None,
        None,
        false,
    );

    let options = ClientInitResponseOptions {
        hash_algorithm: Some(HashAlgorithm::None),
        ..Default::default()
    };

    GCIRFormatter::generate_v1_format(&mut ctx, &options).expect("should have a response")
}
