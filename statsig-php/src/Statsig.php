<?php

namespace Statsig;

use Statsig\EvaluationTypes\DynamicConfig;
use Statsig\EvaluationTypes\Experiment;
use Statsig\EvaluationTypes\FeatureGate;
use Statsig\EvaluationTypes\Layer;
use Statsig\StatsigEventData;

class Statsig
{
    public $__ref = null;

    public function __construct(string $sdk_key, ?StatsigOptions $options = null)
    {
        $options_ref = $options ? $options->__ref : (new StatsigOptions)->__ref;

        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_create($sdk_key, $options_ref);
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        $this->flushEvents();
        StatsigFFI::get()->statsig_release($this->__ref);
        $this->__ref = null;
    }

    public function initialize(): void
    {
        StatsigFFI::get()->statsig_initialize_blocking($this->__ref);
    }

    public function flushEvents(): void
    {
        StatsigFFI::get()->statsig_flush_events_blocking($this->__ref);
    }

    public function logEvent(
        StatsigEventData $event_data,
        StatsigUser $user
    ): void {
        $data = json_encode($event_data);
        StatsigFFI::get()->statsig_log_event($this->__ref, $user->__ref, $data);
    }

    public function getClientInitializeResponse(StatsigUser $user, ?array $options = null): string
    {
        return StatsigFFI::get()->statsig_get_client_init_response($this->__ref, $user->__ref, encode_or_null($options));
    }

    /**
     * Feature Gate Functions
     */

    public function checkGate(StatsigUser $user, string $name, ?array $options = null): bool
    {
        return StatsigFFI::get()->statsig_check_gate($this->__ref, $user->__ref, $name, encode_or_null($options));
    }

    public function getFeatureGate(StatsigUser $user, string $name, ?array $options = null): FeatureGate
    {
        $raw_result = StatsigFFI::get()->statsig_get_feature_gate($this->__ref, $user->__ref, $name, encode_or_null($options));
        return new FeatureGate($raw_result);
    }

    public function manuallyLogGateExposure(StatsigUser $user, string $name): void
    {
        StatsigFFI::get()->statsig_manually_log_gate_exposure($this->__ref, $user->__ref, $name);
    }

    /**
     * Dynamic Config Functions
     */

    public function getDynamicConfig(StatsigUser $user, string $name, ?array $options = null): DynamicConfig
    {
        $raw_result = StatsigFFI::get()->statsig_get_dynamic_config($this->__ref, $user->__ref, $name, encode_or_null($options));
        return new DynamicConfig($raw_result);
    }

    public function manuallyLogDynamicConfigExposure(StatsigUser $user, string $name): void
    {
        StatsigFFI::get()->statsig_manually_log_dynamic_config_exposure($this->__ref, $user->__ref, $name);
    }

    /**
     * Experiment Functions
     */

    public function getExperiment(StatsigUser $user, string $name, ?array $options = null): Experiment
    {
        $raw_result = StatsigFFI::get()->statsig_get_experiment($this->__ref, $user->__ref, $name, encode_or_null($options));
        return new Experiment($raw_result);
    }

    public function manuallyLogExperimentExposure(StatsigUser $user, string $name): void
    {
        StatsigFFI::get()->statsig_manually_log_experiment_exposure($this->__ref, $user->__ref, $name);
    }

    /**
     * Layer Functions
     */

    public function getLayer(StatsigUser $user, string $name, ?array $options = null): Layer
    {
        $raw_result = StatsigFFI::get()->statsig_get_layer($this->__ref, $user->__ref, $name, encode_or_null($options));
        return new Layer($raw_result, $this->__ref);
    }

    public function manuallyLogLayerParameterExposure(StatsigUser $user, string $layer_name, string $param_name): void
    {
        StatsigFFI::get()->statsig_manually_log_layer_parameter_exposure($this->__ref, $user->__ref, $layer_name, $param_name);
    }
}

function encode_or_null(?array $options): ?string
{
    return is_null($options) ? null : json_encode($options);
}
