<?php

namespace Statsig\EvaluationTypes;

class FeatureGate extends BaseEvaluation
{
    public readonly bool $value;

    public function __construct(string $raw_result)
    {
        $result = json_decode($raw_result, true);
        parent::__construct($raw_result, $result);
        $this->value = (bool)($result['value'] ?? false);
    }
}
