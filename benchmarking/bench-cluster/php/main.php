<?php

$sdkVariant = getenv('SDK_VARIANT');

if ($sdkVariant == 'core') {
    $loader = require __DIR__ . '/vendor/autoload.php';
    // Both SDKs export Statsig\\ classes; explicitly select the core namespace.
    $loader->setPsr4('Statsig\\', __DIR__ . '/vendor/statsig/statsig-php-core/src');
    require_once __DIR__ . '/BenchCore.php';
    BenchCore::run();
} else {
    exec("rm -rf " . __DIR__ . "/vendor/statsig/statsig-php-core"); // remove the core sdk to avoid name conflicts
    require_once __DIR__ . '/vendor/autoload.php';
    require_once __DIR__ . '/BenchLegacy.php';
    BenchLegacy::run();
}
