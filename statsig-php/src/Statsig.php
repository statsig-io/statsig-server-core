<?php

namespace Statsig;

use Statsig\EvaluationTypes\DynamicConfig;
use Statsig\EvaluationTypes\Experiment;
use Statsig\EvaluationTypes\FeatureGate;
use Statsig\EvaluationTypes\Layer;
use Statsig\StatsigEventData;

class Statsig
{
    public $__ref = null; // phpcs:ignore

    protected $is_shutdown = false;

    public function __construct(string $sdk_key, ?StatsigOptions $options = null)
    {
        $options_ref = $options ? $options->__ref : (new StatsigOptions())->__ref;

        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_create($sdk_key, $options_ref);
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        $this->shutdown();

        StatsigFFI::get()->statsig_release($this->__ref);
        $this->__ref = null;
    }

    public function initialize(): void
    {
        StatsigFFI::get()->statsig_initialize_blocking($this->__ref);
    }

    public function shutdown(): void
    {
        if ($this->is_shutdown) {
            return;
        }

        StatsigFFI::get()->statsig_shutdown_blocking($this->__ref);
        $this->is_shutdown = true;
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
        $ptr = StatsigFFI::get()->statsig_get_client_init_response(
            $this->__ref,
            $user->__ref,
            encode_or_null($options)
        );

        return StatsigFFI::takeString($ptr);
    }

    public function identify(StatsigUser $user): void
    {
        StatsigFFI::get()->statsig_identify($this->__ref, $user->__ref);
    }

    /**
     * Feature Gate Functions
     */

    public function checkGate(StatsigUser $user, string $name, ?array $options = null): bool
    {
        return StatsigFFI::get()->statsig_check_gate(
            $this->__ref,
            $user->__ref,
            $name,
            encode_or_null($options)
        );
    }

    public function getFeatureGate(StatsigUser $user, string $name, ?array $options = null): FeatureGate
    {
        $ptr = StatsigFFI::get()->statsig_get_feature_gate(
            $this->__ref,
            $user->__ref,
            $name,
            encode_or_null($options)
        );

        $raw_result = StatsigFFI::takeString($ptr);

        return new FeatureGate($raw_result);
    }

    public function manuallyLogGateExposure(StatsigUser $user, string $name): void
    {
        StatsigFFI::get()->statsig_manually_log_gate_exposure(
            $this->__ref,
            $user->__ref,
            $name
        );
    }

    /**
     * Dynamic Config Functions
     */

    public function getDynamicConfig(StatsigUser $user, string $name, ?array $options = null): DynamicConfig
    {
        $ptr = StatsigFFI::get()->statsig_get_dynamic_config(
            $this->__ref,
            $user->__ref,
            $name,
            encode_or_null($options)
        );

        $raw_result = StatsigFFI::takeString($ptr);
        return new DynamicConfig($raw_result);
    }

    public function manuallyLogDynamicConfigExposure(StatsigUser $user, string $name): void
    {
        StatsigFFI::get()->statsig_manually_log_dynamic_config_exposure(
            $this->__ref,
            $user->__ref,
            $name
        );
    }

    /**
     * Experiment Functions
     */

    public function getExperiment(StatsigUser $user, string $name, ?array $options = null): Experiment
    {
        $ptr = StatsigFFI::get()->statsig_get_experiment(
            $this->__ref,
            $user->__ref,
            $name,
            encode_or_null($options)
        );

        $raw_result = StatsigFFI::takeString($ptr);
        return new Experiment($raw_result);
    }

    public function manuallyLogExperimentExposure(StatsigUser $user, string $name): void
    {
        StatsigFFI::get()->statsig_manually_log_experiment_exposure(
            $this->__ref,
            $user->__ref,
            $name
        );
    }

    /**
     * Layer Functions
     */

    public function getLayer(StatsigUser $user, string $name, ?array $options = null): Layer
    {
        $ptr = StatsigFFI::get()->statsig_get_layer(
            $this->__ref,
            $user->__ref,
            $name,
            encode_or_null($options)
        );

        $raw_result = StatsigFFI::takeString($ptr);
        return new Layer($raw_result, $this->__ref);
    }

    public function manuallyLogLayerParameterExposure(StatsigUser $user, string $layer_name, string $param_name): void
    {
        StatsigFFI::get()->statsig_manually_log_layer_parameter_exposure(
            $this->__ref,
            $user->__ref,
            $layer_name,
            $param_name
        );
    }

    /**
     * Override Functions
     */

    public function overrideGate(string $gateName, bool $value, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_override_gate(
            $this->__ref,
            $gateName,
            $value,
            $id
        );
    }

    public function overrideDynamicConfig(string $configName, array $value, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_override_dynamic_config(
            $this->__ref,
            $configName,
            json_encode($value),
            $id
        );
    }

    public function overrideExperiment(string $experimentName, array $value, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_override_experiment(
            $this->__ref,
            $experimentName,
            json_encode($value),
            $id
        );
    }

    public function overrideExperimentByGroupName(string $experimentName, string $groupName, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_override_experiment_by_group_name(
            $this->__ref,
            $experimentName,
            $groupName,
            $id
        );
    }

    public function overrideLayer(string $layerName, array $value, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_override_layer(
            $this->__ref,
            $layerName,
            json_encode($value),
            $id
        );
    }

    public function removeGateOverride(string $gateName, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_remove_gate_override(
            $this->__ref,
            $gateName,
            $id
        );
    }

    public function removeDynamicConfigOverride(string $configName, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_remove_dynamic_config_override(
            $this->__ref,
            $configName,
            $id
        );
    }

    public function removeExperimentOverride(string $experimentName, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_remove_experiment_override(
            $this->__ref,
            $experimentName,
            $id
        );
    }

    public function removeLayerOverride(string $layerName, ?string $id = null): void
    {
        StatsigFFI::get()->statsig_remove_layer_override(
            $this->__ref,
            $layerName,
            $id
        );
    }

    public function removeAllOverrides(): void
    {
        StatsigFFI::get()->statsig_remove_all_overrides($this->__ref);
    }
}

function encode_or_null(?array $options): ?string
{
    return is_null($options) ? null : json_encode($options);
}
