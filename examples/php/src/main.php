<?php

require_once "vendor/autoload.php";

use Statsig\StatsigFFI\Statsig;
use Statsig\StatsigFFI\StatsigUser;

$statsig = new Statsig("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW");
$statsig->initialize(function () use ($statsig) {
    $user = new StatsigUser("a-user", "daniel@statsig.com");
    $gcir = $statsig->getClientInitializeResponse($user);
    echo $gcir;
});


while (true) {
    sleep(10);
}