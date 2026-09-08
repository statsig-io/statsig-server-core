<?php

namespace Statsig;

use FFI;
use Statsig\EvaluationTypes\DynamicConfig;
use Statsig\EvaluationTypes\Experiment;
use Statsig\EvaluationTypes\ExperimentGroupsResult;
use Statsig\EvaluationTypes\FeatureGate;
use Statsig\EvaluationTypes\Layer;
use Statsig\StatsigEventData;

class Statsig
{
    public $__ref = null; // phpcs:ignore

    protected $is_shutdown = false;

    /** @var array<int, int> native refs of instances that have not been released yet */
    private static $live_refs = [];

    private static $teardown_registered = false;

    private static $runtime_released = false;

    public function __construct(string $sdk_key, ?StatsigOptions $options = null)
    {
        $options_ref = $options ? $options->__ref : (new StatsigOptions())->__ref;

        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_create($sdk_key, $options_ref);

        self::trackForTeardown($this->__ref);
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        // The teardown hook already shut this instance down and released it.
        if (self::$runtime_released) {
            $this->__ref = null;
            return;
        }

        $this->shutdown();

        StatsigFFI::get()->statsig_release($this->__ref);
        unset(self::$live_refs[$this->__ref]);
        $this->__ref = null;
    }

    /**
     * The shared tokio runtime outlives every instance, so releasing an
     * instance is not enough: PHP unloads the FFI library at the end of each
     * request, and on Windows that unmaps the module while the runtime's
     * worker threads are still running inside it. Stopping the runtime during
     * request teardown is what keeps those threads from outliving the module.
     */
    private static function trackForTeardown($ref): void
    {
        self::$live_refs[$ref] = $ref;

        if (self::$teardown_registered) {
            return;
        }

        self::$teardown_registered = true;
        register_shutdown_function(function () {
            self::releaseSharedRuntime();
        });
    }

    private static function releaseSharedRuntime(): void
    {
        if (self::$runtime_released) {
            return;
        }

        self::$runtime_released = true;
        $ffi = StatsigFFI::get();

        // Shutdown callbacks run before object destructors, so any instance
        // still alive has not flushed yet. Flush it here, before the runtime
        // that would do the flushing is stopped below.
        foreach (self::$live_refs as $ref) {
            $ffi->statsig_shutdown_blocking($ref);
            $ffi->statsig_release($ref);
        }

        self::$live_refs = [];

        $ffi->statsig_shutdown_shared_runtime();
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

    /**
     * Supported $options keys:
     * - disable_exposure_logging (bool)
     * - user_persisted_values (array): map of config name to sticky values, as
     *   saved through PersistentStorage. Only honored when a persistent
     *   storage adapter is configured on StatsigOptions.
     * - enforce_overrides (bool): when a persisted sticky value exists, let a
     *   matching console override rule take precedence over it.
     * - enforce_targeting (bool): when a persisted sticky value exists,
     *   re-check targeting and drop the sticky value if the user no longer
     *   passes targeting.
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

    public function getExperimentByGroupName(string $experimentName, string $groupName): Experiment
    {
        $len = StatsigFFI::get()->new("uint64_t");
        $ptr = StatsigFFI::get()->statsig_get_raw_experiment_by_group_name(
            $this->__ref,
            $experimentName,
            $groupName,
            FFI::addr($len)
        );

        return new Experiment(StatsigFFI::takeString($ptr));
    }

    public function getExperimentByGroupIdAdvanced(string $experimentName, string $groupId): Experiment
    {
        $len = StatsigFFI::get()->new("uint64_t");
        $ptr = StatsigFFI::get()->statsig_get_raw_experiment_by_group_id_advanced(
            $this->__ref,
            $experimentName,
            $groupId,
            FFI::addr($len)
        );

        return new Experiment(StatsigFFI::takeString($ptr));
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
     * Returns the experiment's active state and the group name, rule id, id type, and
     * return value for each of its groups, without requiring a user evaluation.
     *
     * `isExperimentActive` is null if the name does not refer to an experiment (unknown
     * name or a non-experiment entity like a dynamic config or autotune); otherwise it
     * reflects the experiment's isActive state, and `groups` contains the experiment's
     * groups regardless of that state. Rules that are not experiment groups (e.g.
     * holdout or sizing rules) are excluded.
     */
    public function getExperimentGroups(string $experimentName): ExperimentGroupsResult
    {
        $len = StatsigFFI::get()->new("uint64_t");
        $ptr = StatsigFFI::get()->statsig_get_experiment_groups(
            $this->__ref,
            $experimentName,
            FFI::addr($len)
        );

        if (\FFI::isNull($ptr)) {
            return new ExperimentGroupsResult('');
        }

        return new ExperimentGroupsResult(StatsigFFI::takeString($ptr));
    }

    /**
     * Layer Functions
     */

    /**
     * Supported $options keys: see getExperiment().
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
     * Entity List Functions
     */

    public function getFeatureGateList(): array
    {
        $ffi = StatsigFFI::get();
        // $len satisfies the required uint64_t* out-param; its written value is
        // unused because takeString reads the NUL-terminated JSON payload.
        $len = $ffi->new('uint64_t');
        $ptr = $ffi->statsig_get_feature_gate_list($this->__ref, \FFI::addr($len));

        if (\FFI::isNull($ptr)) {
            return [];
        }

        $raw_result = StatsigFFI::takeString($ptr);
        $decoded = json_decode($raw_result, true);
        return is_array($decoded) ? $decoded : [];
    }

    public function getDynamicConfigList(): array
    {
        $ffi = StatsigFFI::get();
        // $len satisfies the required uint64_t* out-param; its written value is
        // unused because takeString reads the NUL-terminated JSON payload.
        $len = $ffi->new('uint64_t');
        $ptr = $ffi->statsig_get_dynamic_config_list($this->__ref, \FFI::addr($len));

        if (\FFI::isNull($ptr)) {
            return [];
        }

        $raw_result = StatsigFFI::takeString($ptr);
        $decoded = json_decode($raw_result, true);
        return is_array($decoded) ? $decoded : [];
    }

    public function getExperimentList(): array
    {
        $ffi = StatsigFFI::get();
        // $len satisfies the required uint64_t* out-param; its written value is
        // unused because takeString reads the NUL-terminated JSON payload.
        $len = $ffi->new('uint64_t');
        $ptr = $ffi->statsig_get_experiment_list($this->__ref, \FFI::addr($len));

        if (\FFI::isNull($ptr)) {
            return [];
        }

        $raw_result = StatsigFFI::takeString($ptr);
        $decoded = json_decode($raw_result, true);
        return is_array($decoded) ? $decoded : [];
    }

    public function getAutotuneList(): array
    {
        $ffi = StatsigFFI::get();
        // $len satisfies the required uint64_t* out-param; its written value is
        // unused because takeString reads the NUL-terminated JSON payload.
        $len = $ffi->new('uint64_t');
        $ptr = $ffi->statsig_get_autotune_list($this->__ref, \FFI::addr($len));

        if (\FFI::isNull($ptr)) {
            return [];
        }

        $raw_result = StatsigFFI::takeString($ptr);
        $decoded = json_decode($raw_result, true);
        return is_array($decoded) ? $decoded : [];
    }

    public function getLayerList(): array
    {
        $ffi = StatsigFFI::get();
        // $len satisfies the required uint64_t* out-param; its written value is
        // unused because takeString reads the NUL-terminated JSON payload.
        $len = $ffi->new('uint64_t');
        $ptr = $ffi->statsig_get_layer_list($this->__ref, \FFI::addr($len));

        if (\FFI::isNull($ptr)) {
            return [];
        }

        $raw_result = StatsigFFI::takeString($ptr);
        $decoded = json_decode($raw_result, true);
        return is_array($decoded) ? $decoded : [];
    }

    public function getParameterStoreList(): array
    {
        $ffi = StatsigFFI::get();
        // $len satisfies the required uint64_t* out-param; its written value is
        // unused because takeString reads the NUL-terminated JSON payload.
        $len = $ffi->new('uint64_t');
        $ptr = $ffi->statsig_get_parameter_store_list($this->__ref, \FFI::addr($len));

        if (\FFI::isNull($ptr)) {
            return [];
        }

        $raw_result = StatsigFFI::takeString($ptr);
        $decoded = json_decode($raw_result, true);
        return is_array($decoded) ? $decoded : [];
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
