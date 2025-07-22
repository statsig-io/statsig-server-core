<?php

use Statsig\StatsigServer;
use Statsig\StatsigOptions;
use Statsig\Adapters\LocalFileDataAdapter;
use Statsig\ConfigSpecs;
use Statsig\StatsigNetwork;
use Statsig\StatsigUser;
use Statsig\StatsigEvent;
use Statsig\IDList;
use Statsig\Adapters\ILoggingAdapter;
use Statsig\Adapters\ISpecsAdapter;



class BenchLegacy {
    static $SCRAPI_URL = "http://scrapi:8000";
    static $ITER_LITE = 1000;
    static $ITER_HEAVY = 10_000;
    static $SDK_TYPE = "php-server";

    public static function run() {
        $sdkVersion = self::getSdkVersion();
        echo "Statsig PHP Legacy (v" . $sdkVersion . ")\n";
        echo "--------------------------------\n";

        $specNames = self::loadSpecNames();

        $network = new StatsigNetwork();
        $network->setSdkKey("secret-PHP_LEGACY");

        $client = getPrivateField($network, 'client');
        $config = getPrivateField($client, 'config');
        $config['base_uri'] = self::$SCRAPI_URL . "/v1/";
        setPrivateField($client, 'config', $config);

        $config_adapter = new LocalFileDataAdapter("/shared-volume/php-legacy");
        ConfigSpecs::sync($config_adapter, $network);
        IDList::sync($config_adapter, $network);

        $logging_adapter = new CustomLoggingAdapter($network);
        $options = new StatsigOptions($config_adapter, $logging_adapter);

        $statsig = new StatsigServer("secret-PHP_LEGACY", $options);

        $globalUser = StatsigUser::withUserID("global_user");

        $results = [];

        foreach ($specNames['feature_gates'] as $gate) {
            ConfigSpecs::sync($config_adapter, $network);
            IDList::sync($config_adapter, $network);
            
            $results[] = self::benchmark("check_gate", $gate, self::$ITER_HEAVY, function() use ($statsig, $gate) {
                $result = $statsig->checkGate(self::createUser(), $gate);
                if ($gate === "test_public" && $result === false) {
                    throw new Exception("test_public is false");
                }
            });

            $results[] = self::benchmark("check_gate_global_user", $gate, self::$ITER_HEAVY, function() use ($statsig, $gate, $globalUser)  {
                $statsig->checkGate($globalUser, $gate);
            });

            $results[] = self::benchmark("get_feature_gate", $gate, self::$ITER_HEAVY, function() use ($statsig, $gate) {
                    $statsig->getFeatureGate(self::createUser(), $gate);
                });

            $results[] = self::benchmark("get_feature_gate_global_user", $gate, self::$ITER_HEAVY, function() use ($statsig, $gate, $globalUser) {
                $statsig->getFeatureGate($globalUser, $gate);
            });
        }

        foreach ($specNames['dynamic_configs'] as $config) {
            ConfigSpecs::sync($config_adapter, $network);
            IDList::sync($config_adapter, $network);
          
            $results[] = self::benchmark("get_dynamic_config", $config, self::$ITER_HEAVY, function() use ($statsig, $config) {
                $statsig->getConfig(self::createUser(), $config);
            });

            $results[] = self::benchmark("get_dynamic_config_global_user", $config, self::$ITER_HEAVY, function() use ($statsig, $config, $globalUser) {
                $statsig->getConfig($globalUser, $config);
            });
        }

        foreach ($specNames['experiments'] as $experiment) {
            ConfigSpecs::sync($config_adapter, $network);
            $results[] = self::benchmark("get_experiment", $experiment, self::$ITER_HEAVY, function() use ($statsig, $experiment) {
                $statsig->getExperiment(self::createUser(), $experiment);
            });

            $results[] = self::benchmark("get_experiment_global_user", $experiment, self::$ITER_HEAVY, function() use ($statsig, $experiment, $globalUser) {
                $statsig->getExperiment($globalUser, $experiment);
            });
        }

        foreach ($specNames['layers'] as $layer) {
            ConfigSpecs::sync($config_adapter, $network);
            IDList::sync($config_adapter, $network);
         
            $results[] = self::benchmark("get_layer", $layer, self::$ITER_HEAVY, function() use ($statsig, $layer) {
                $statsig->getLayer(self::createUser(), $layer);
            });

            $results[] = self::benchmark("get_layer_global_user", $layer, self::$ITER_HEAVY, function() use ($statsig, $layer, $globalUser) {
                $statsig->getLayer($globalUser, $layer);
            });
        }

        ConfigSpecs::sync($config_adapter, $network);
        IDList::sync($config_adapter, $network);
      
        $results[] = self::benchmark("get_client_initialize_response", "n/a", self::$ITER_LITE, function() use ($statsig) {
            $statsig->getClientInitializeResponse(self::createUser());
        });

        $results[] = self::benchmark("get_client_initialize_response_global_user", "n/a", self::$ITER_LITE, function() use ($statsig, $globalUser) {
            $statsig->getClientInitializeResponse($globalUser);
        });

        // $statsig->shutdown();

        self::writeResults($results, $sdkVersion);
    }

    function loadSpecNames() {
        $path = "/shared-volume/spec_names.json";
        for ($i = 0; $i < 10; $i++) {
            if (file_exists($path)) { break; }
            echo "Waiting for spec_names.json to be created...\n";
            sleep(1);
        }
        echo "Loading spec_names.json...\n";
        $content = file_get_contents($path);
        $json = json_decode($content, true);
        echo "JSON: " . count($json['feature_gates']) . "\n";
        return $json;
    }

    function createUser() {
        return StatsigUser::withUserID("user_" . rand(0, 1000000))
            ->setEmail("user@example.com")
            ->setCustom([
                "isAdmin" => false,
            ])
            ->setPrivateAttributes([
                "isPaid" => "nah",
            ])
            ->setCountry("US")
            ->setLocale("en-US")
            ->setAppVersion("1.0.0")
            ->setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            ->setIP("127.0.0.1");
    }

    function benchmark(string $benchmarkName, string $specName, int $iterations, callable $func) {
        $durations = [];
        for ($i = 0; $i < $iterations; $i++) {
            $startMs = microtime(true) * 1000;
            $func();
            $endMs = microtime(true) * 1000;
            $durations[] = $endMs - $startMs;
        }

        sort($durations);
        $p99 = $durations[intval($iterations * 0.99)];
        $max = $durations[count($durations) - 1];
        $min = $durations[0];
        $median = $durations[intval($iterations / 2)];
        $avg = array_sum($durations) / $iterations;

        assert($min <= $max);

        $sdkVersion = self::getSdkVersion();

        $result = [
            "benchmarkName" => $benchmarkName,
            "p99" => $p99,
            "max" => $max,
            "min" => $min,
            "median" => $median,
            "avg" => $avg,
            "specName" => $specName,
            "sdkType" => self::$SDK_TYPE,
            "sdkVersion" => $sdkVersion,
        ];

        echo str_pad($benchmarkName, 30) . " p99(" . number_format($p99, 4) . "ms) max(" . number_format($max, 4) . "ms) " . $specName . "\n";

        return $result;
    }

    function getSdkVersion() {
        $installedPackages = json_decode(file_get_contents(__DIR__ . '/vendor/composer/installed.json'), true);
        $statsigCoreVersion = null;

        foreach ($installedPackages['packages'] as $package) {
            if ($package['name'] === 'statsig/statsig-php-core') {
                $statsigCoreVersion = $package['version'];
                break;
            }
        }

        return $statsigCoreVersion;
    }

    function writeResults(array $results, string $sdkVersion) {
        $path = "/shared-volume/" . self::$SDK_TYPE . "-" . $sdkVersion . "-results.json";
        file_put_contents($path, json_encode([
            "sdkType" => self::$SDK_TYPE,
            "sdkVersion" => $sdkVersion,
            "results" => $results,
        ], JSON_PRETTY_PRINT));
    }
}

function getPrivateField(object $instance, string $field) {
    $reflection = new ReflectionClass($instance);
    $property = $reflection->getProperty($field);
    $property->setAccessible(true);  // Bypass visibility
    return $property->getValue($instance);
}

function setPrivateField(object $instance, string $field, $value) {
    $reflection = new ReflectionClass($instance);
    $property = $reflection->getProperty($field);
    $property->setAccessible(true);  // Bypass visibility
    $property->setValue($instance, $value);
}

class CustomLoggingAdapter implements ILoggingAdapter
{
    public bool $disabled = true;
    private StatsigNetwork $network;

    public function __construct(StatsigNetwork $network)
    {
        $this->network = $network;
    }

    public function enqueueEvents(array $events)
    {
        if ($this->disabled) {
            return;
        }

        $this->network->logEvents($events);
    }

    public function getQueuedEvents(): array
    {
        return [];
    }

    public function shutdown() {}
}
