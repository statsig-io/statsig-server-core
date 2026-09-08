<?php

/**
 * Child process for SharedRuntimeTeardownTest. Models one request: build a
 * Statsig, log an event, and leave teardown to PHP's shutdown sequence.
 *
 * Usage: php shared_runtime_teardown_child.php <mode> <mock_server_url>
 */

require_once dirname(__FILE__) . '/../../vendor/autoload.php';

use Statsig\Statsig;
use Statsig\StatsigEventData;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

$mode = $argv[1] ?? '';
$server_url = $argv[2] ?? '';

if ($mode === 'no_instance') {
    exit(0);
}

$options = new StatsigOptions(
    specs_url: $server_url . '/v2/download_config_specs',
    log_event_url: $server_url . '/v1/log_event',
    output_log_level: 'none',
);

$statsig = new Statsig('secret-key', $options);
$statsig->initialize();

$statsig->logEvent(new StatsigEventData('teardown_probe'), new StatsigUser('test_user'));

if ($mode === 'explicit_shutdown') {
    $statsig->shutdown();
}

// 'never_shutdown' deliberately falls off the end holding a live instance,
// which is what a normal page does.
exit(0);
