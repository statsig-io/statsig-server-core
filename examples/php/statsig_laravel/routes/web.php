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
        $experiments[$name] = $statsig->getExperiment($user, $name);
    }

    return response()->json($experiments);
});

Route::get('/statsig/get_layer', function (Statsig $statsig) {
    $user_id = request()->query('user_id', null);
    if ($user_id === null) {
        return response()->json(["error" => "user_id is required"]);
    }

    $layer_name = request()->query('layer_name', null);
    if ($layer_name === null) {
        return response()->json(["error" => "layer_name is required"]);
    }

    $param_name = request()->query('param_name', null);
    if ($param_name === null) {
        return response()->json(["error" => "param_name is required"]);
    }

    $user = new StatsigUser($user_id);
    $layer = $statsig->getLayer($user, $layer_name);
    $value = $layer->get($param_name, null);


    return response()->json($value);
});


Route::get('/statsig/log_event', function (Statsig $statsig) {
    $user = new StatsigUser("user_id");
    $statsig->logEvent(new StatsigEventData("test_event"), $user);
    return response()->json(["success" => true]);
});
