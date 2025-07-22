<?php

require_once __DIR__ . '/vendor/autoload.php';


$sdkVariant = getenv('SDK_VARIANT');

if ($sdkVariant == 'core') {
    require_once __DIR__ . '/BenchCore.php';
    BenchCore::run();
} else {
    require_once __DIR__ . '/BenchLegacy.php';
    BenchLegacy::run();
}
