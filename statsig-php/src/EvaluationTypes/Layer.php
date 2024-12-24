<?php

namespace Statsig\EvaluationTypes;

use Statsig\StatsigFFI;

class Layer extends BaseEvaluation
{
    public readonly ?string $groupName;
    public readonly ?string $allocatedExperimentName;
    private readonly array $__value;
    private readonly string $__statsig_ref;

    public function __construct(string $raw_result, string $__statsig_ref)
    {
        $result = json_decode($raw_result, true);
        parent::__construct($raw_result, $result);
        $this->__statsig_ref = $__statsig_ref;
        $this->__value = $result['__value'] ?? [];
        $this->groupName = $result['group_name'] ?? null;
        $this->allocatedExperimentName = $result['allocated_experiment_name'] ?? null;
    }

    public function get(string $param_name, $fallback)
    {
        return $this->getValueImpl(
            $this->__value,
            $param_name,
            $fallback,
            function ($param_name) {
                $this->logParameterExposure($param_name);
            }
        );
    }

    private function logParameterExposure(string $param_name): void
    {
        StatsigFFI::get()->statsig_log_layer_param_exposure($this->__statsig_ref, $this->__raw_result, $param_name);
    }
}
