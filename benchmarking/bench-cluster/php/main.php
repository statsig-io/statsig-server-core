<?php

$sdkVariant = getenv('SDK_VARIANT');

if ($sdkVariant == 'core') {
    require_once __DIR__ . '/vendor/autoload.php';
    require_once __DIR__ . '/BenchCore.php';
    BenchCore::run();
} else {
    exec("rm -rf " . __DIR__ . "/vendor/statsig/statsig-php-core"); // remove the core sdk to avoid name conflicts
    require_once __DIR__ . '/vendor/autoload.php';
    require_once __DIR__ . '/BenchLegacy.php';
    BenchLegacy::run();
}
