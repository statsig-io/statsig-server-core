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
use statsig_rust::{evaluation::evaluator_context::IdListResolution, StatsigUser};
use tokio::time::Instant;

use crate::utils::gcir::{generate_v1_response, load_specs_data};

#[test]
fn test_gcir() {
    let specs_data = load_specs_data("eval_proj_dcs.json");

    let id_lists = HashMap::new();
    let user = direct_gcir_user();
    let response = generate_v1_response(
        &user,
        &specs_data,
        IdListResolution::MapLookup(&id_lists),
        None,
    )
    .expect("should have a response");

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

    let user = direct_gcir_user();
    let response = generate_v1_response(
        &user,
        &specs_data,
        IdListResolution::Callback(&id_lists_callback),
        None,
    )
    .expect("should have a response");
    let gate = response
        .feature_gates
        .get("test_id_list")
        .expect("should have a gate");

    assert!(gate.value);
}

#[test]
fn test_enriched_id_list_memberships_are_authoritative_for_gcir() {
    let specs_data = load_specs_data("eval_proj_dcs.json");
    let always_false = |_: &str, _: &str| false;

    let mut member = StatsigUser::with_user_id("dan_1");
    member.set_custom_ids(HashMap::from([(
        "companyID".to_string(),
        "123".to_string(),
    )]));
    Arc::make_mut(&mut member.data).id_list_memberships = Some(HashMap::from([(
        "pmWkWSBC".to_string(),
        HashSet::from(["company_id_list".to_string()]),
    )]));

    let response = generate_v1_response(
        &member,
        &specs_data,
        IdListResolution::Callback(&always_false),
        None,
    )
    .expect("should have a response");
    assert!(response.feature_gates["test_id_list"].value);

    Arc::make_mut(&mut member.data).id_list_memberships = Some(HashMap::new());
    let response = generate_v1_response(
        &member,
        &specs_data,
        IdListResolution::Callback(&|_, _| true),
        None,
    )
    .expect("should have a response");
    assert!(!response.feature_gates["test_id_list"].value);
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
            let user = direct_gcir_user();

            if i % 2 == 0 {
                let res = generate_v1_response(
                    &user,
                    &eval_proj_dcs,
                    IdListResolution::MapLookup(&id_lists),
                    None,
                )
                .expect("should have a response");
                if res.dynamic_configs.len() == 65 {
                    success_count.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                let res = generate_v1_response(
                    &user,
                    &big_number_dcs,
                    IdListResolution::MapLookup(&id_lists),
                    None,
                )
                .expect("should have a response");
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

fn direct_gcir_user() -> StatsigUser {
    let mut user = StatsigUser::with_user_id("dan_1");
    user.set_custom_ids(HashMap::from([(
        "companyID".to_string(),
        "123".to_string(),
    )]));
    user
}
