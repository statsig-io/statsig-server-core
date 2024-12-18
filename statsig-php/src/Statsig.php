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

    public function __construct(string $sdk_key, StatsigOptions $options = null)
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

    public function checkGate(string $name, StatsigUser $user): bool
    {
        return StatsigFFI::get()->statsig_check_gate($this->__ref, $user->__ref, $name);
    }

    public function getFeatureGate(string $name, StatsigUser $user): FeatureGate
    {
        $raw_result = StatsigFFI::get()->statsig_get_feature_gate($this->__ref, $user->__ref, $name);
        return new FeatureGate($raw_result);
    }

    public function getDynamicConfig(string $name, StatsigUser $user): DynamicConfig
    {
        $raw_result = StatsigFFI::get()->statsig_get_dynamic_config($this->__ref, $user->__ref, $name);
        return new DynamicConfig($raw_result);
    }

    public function getExperiment(string $name, StatsigUser $user): Experiment
    {
        $raw_result = StatsigFFI::get()->statsig_get_experiment($this->__ref, $user->__ref, $name);
        return new Experiment($raw_result);
    }

    public function getLayer(string $name, StatsigUser $user): Layer
    {
        $raw_result = StatsigFFI::get()->statsig_get_layer($this->__ref, $user->__ref, $name);
        return new Layer($raw_result, $this->__ref);
    }

    public function getClientInitializeResponse(StatsigUser $user): string
    {
        return StatsigFFI::get()->statsig_get_client_init_response($this->__ref, $user->__ref);
    }

    public function logEvent(
        StatsigEventData $event_data,
        StatsigUser $user,
    ): void {
        $data = json_encode($event_data);
        StatsigFFI::get()->statsig_log_event($this->__ref, $user->__ref, $data);
    }
}
