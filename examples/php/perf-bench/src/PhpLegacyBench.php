<?php

require_once __DIR__ . "/../vendor/autoload.php";

use Statsig\StatsigServer;
use Statsig\StatsigOptions;
use Statsig\Adapters\LocalFileDataAdapter;
use Statsig\ConfigSpecs;
use Statsig\StatsigNetwork;
use Statsig\StatsigUser;
use Statsig\StatsigEvent;
use Statsig\IDList;
use Statsig\Adapters\ILoggingAdapter;

class CustomLoggingAdapter implements ILoggingAdapter
{
    public bool $disabled = true;

    public function enqueueEvents(array $events)
    {
        if ($this->disabled) {
            return;
        }

        $network = new StatsigNetwork();
        $network->setSdkKey(getenv("PERF_SDK_KEY"));
        $network->logEvents($events);
    }

    public function getQueuedEvents(): array
    {
        return [];
    }

    public function shutdown() {}
}


$installedPackages = json_decode(file_get_contents(__DIR__ . '/../vendor/composer/installed.json'), true);
$statsigCoreVersion = null;

foreach ($installedPackages['packages'] as $package) {
    if ($package['name'] === 'statsig/statsigsdk') {
        $statsigCoreVersion = $package['version'];
        break;
    }
}

$sdkType = 'php-server';
$sdkVersion = $statsigCoreVersion ?? 'unknown';

$metadataFile = getenv("BENCH_METADATA_FILE");
file_put_contents($metadataFile, json_encode([
    'sdk_type' => $sdkType,
    'sdk_version' => $sdkVersion,
]));

echo "Statsig PHP Legacy (v" . ($sdkVersion) . ")\n";
echo "--------------------------------\n";

$key = getenv("PERF_SDK_KEY");


$config_adapter = new LocalFileDataAdapter();
$network = new StatsigNetwork();
$network->setSdkKey($key);
ConfigSpecs::sync($config_adapter, $network);
IDList::sync($config_adapter, $network);

$logging_adapter = new CustomLoggingAdapter();
$options = new StatsigOptions($config_adapter, $logging_adapter);
$options->setEventQueueSize(1000);
$statsig = new StatsigServer($key, $options);


$iterations = 5_000; // other SDKs do 100k, but PHP is too slow
$globalUser = StatsigUser::withUserID("global_user");
$results = [];

function makeRandomUser()
{
    return StatsigUser::withUserID(uniqid());
}

function benchmark($func)
{
    global $iterations, $network, $config_adapter;
    ConfigSpecs::sync($config_adapter, $network);
    IDList::sync($config_adapter, $network);

    $durations = [];

    for ($i = 0; $i < $iterations; $i++) {
        $start = microtime(true);
        $func();
        $end = microtime(true);
        $durations[] = ($end - $start) * 1000; // Convert to milliseconds
    }

    // Calculate p99
    sort($durations);
    $p99Index = floor($iterations * 0.99);
    return $durations[$p99Index];
}

function logBenchmark($name, $p99)
{
    echo str_pad($name, 50) . number_format($p99, 4) . "ms\n";

    $ci = getenv("CI");
    if ($ci !== '1' && $ci !== 'true') {
        return;
    }

    global $statsig, $globalUser, $sdkType, $sdkVersion;

    $event = new StatsigEvent("sdk_benchmark");
    $event->setUser($globalUser);
    $event->setValue($p99);
    $event->setMetadata([
        'benchmarkName' => $name,
        'sdkType' => $sdkType,
        'sdkVersion' => $sdkVersion
    ]);

    $statsig->logEvent($event);
}

function runCheckGate()
{
    global $statsig, $results;
    $p99 = benchmark(function () use ($statsig) {
        $statsig->checkGate(makeRandomUser(), 'test_public');
    });
    $results['check_gate'] = $p99;
}

function runCheckGateGlobalUser()
{
    global $statsig, $results, $globalUser;
    $p99 = benchmark(function () use ($statsig, $globalUser) {
        $statsig->checkGate($globalUser, 'test_public');
    });
    $results['check_gate_global_user'] = $p99;
}

function runGetFeatureGate()
{
    global $statsig, $results;
    $p99 = benchmark(function () use ($statsig) {
        $statsig->getFeatureGate(makeRandomUser(), 'test_public');
    });
    $results['get_feature_gate'] = $p99;
}

function runGetFeatureGateGlobalUser()
{
    global $statsig, $results, $globalUser;
    $p99 = benchmark(function () use ($statsig, $globalUser) {
        $statsig->getFeatureGate($globalUser, 'test_public');
    });
    $results['get_feature_gate_global_user'] = $p99;
}

function runGetExperiment()
{
    global $statsig, $results;
    $p99 = benchmark(function () use ($statsig) {
        $statsig->getExperiment(makeRandomUser(), 'an_experiment');
    });
    $results['get_experiment'] = $p99;
}

function runGetExperimentGlobalUser()
{
    global $statsig, $results, $globalUser;
    $p99 = benchmark(function () use ($statsig, $globalUser) {
        $statsig->getExperiment($globalUser, 'an_experiment');
    });
    $results['get_experiment_global_user'] = $p99;
}

function runGetClientInitializeResponse()
{
    global $statsig, $results;
    $p99 = benchmark(function () use ($statsig) {
        $statsig->getClientInitializeResponse(makeRandomUser());
    });
    $results['get_client_initialize_response'] = $p99;
}

function runGetClientInitializeResponseGlobalUser()
{
    global $statsig, $results, $globalUser;
    $p99 = benchmark(function () use ($statsig, $globalUser) {
        $statsig->getClientInitializeResponse($globalUser);
    });
    $results['get_client_initialize_response_global_user'] = $p99;
}

// Run all benchmarks
runCheckGate();
runCheckGateGlobalUser();
runGetFeatureGate();
runGetFeatureGateGlobalUser();
runGetExperiment();
runGetExperimentGlobalUser();
runGetClientInitializeResponse();
runGetClientInitializeResponseGlobalUser();

$logging_adapter->disabled = false;

// Log results
foreach ($results as $name => $p99) {
    logBenchmark($name, $p99);
}

$statsig->flush();
