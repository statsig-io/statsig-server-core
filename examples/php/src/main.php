<?php

require_once "vendor/autoload.php";

use Statsig\StatsigFFI\Statsig;
use Statsig\StatsigFFI\StatsigUser;

$secret_key = getenv('test_api_key');
$statsig = new Statsig($secret_key);
$statsig->initialize(function () use ($statsig) {
    $user = new StatsigUser("a-user", "daniel@statsig.com");
    $gcir = $statsig->getClientInitializeResponse($user);
    echo $gcir;
});


while (true) {
    sleep(10);
}