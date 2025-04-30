<?php

require_once __DIR__ . '/vendor/autoload.php';

use Statsig\Statsig;
use Statsig\StatsigUser;

$sdk_key = getenv('STATSIG_SERVER_SDK_KEY');
if (!$sdk_key) {
    throw new Exception('STATSIG_SERVER_SDK_KEY is not set');
}

// Initialize Statsig
$statsig = new Statsig($sdk_key);
$statsig->initialize();

// Create a user with system information
$user = new StatsigUser(
    'a_user',
    [],
    null,
    null,
    null,
    null,
    null,
    null,
    [
        'os' => strtolower(PHP_OS),
        'arch' => php_uname('m'),
        'phpVersion' => PHP_VERSION,
    ],
    null
);

// Check gate and get client initialize response
$gate = $statsig->checkGate($user, 'test_public');
$gcir = $statsig->getClientInitializeResponse($user);

echo "-------------------------------- Get Client Initialize Response --------------------------------\n";
echo json_encode(json_decode($gcir), JSON_PRETTY_PRINT) . "\n";
echo "-------------------------------------------------------------------------------------------------\n";

echo "Gate test_public: " . ($gate ? "true" : "false") . "\n";

if (!$gate) {
    throw new Exception('"test_public" gate is false but should be true');
}

$gcir_array = json_decode($gcir, true);
if (count(array_keys($gcir_array)) < 1) {
    throw new Exception('GCIR is missing required fields');
}

echo "All checks passed, shutting down...\n";
$statsig->shutdown();
echo "Shutdown complete\n";
