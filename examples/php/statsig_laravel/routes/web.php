<?php

use Illuminate\Support\Facades\Route;
use Statsig\Statsig;
use Statsig\StatsigUser;
use Statsig\StatsigEventData;

Route::get('/statsig/get_client_init_response', function (Statsig $statsig) {
    $user = new StatsigUser("user_id");

    $response = $statsig->getClientInitializeResponse($user);
    return response($response)->header("Content-Type", "application/json");
});

Route::get('/statsig/list_experiments', function (Statsig $statsig) {
    $user = new StatsigUser("user_id");
    $expNames = ["an_experiment", "another_experiment", "stable_id_experiment"];

    $experiments = [];
    foreach ($expNames as $name) {
        $experiments[$name] = $statsig->getExperiment($name, $user);
    }

    return response()->json($experiments);
});


Route::get('/statsig/log_event', function (Statsig $statsig) {
    $user = new StatsigUser("user_id");
    $statsig->logEvent(new StatsigEventData("test_event"), $user);
    $statsig->flushEvents(function () {
        //
    });

    return response()->json(["success" => true]);
});
