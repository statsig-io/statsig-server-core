<?php

namespace Statsig\EvaluationTypes;

class DynamicConfig extends BaseEvaluation
{
    public readonly array $value;

    public function __construct(string $raw_result)
    {
        $result = json_decode($raw_result, true);
        parent::__construct($raw_result, $result);
        $this->value = $result['value'] ?? [];
    }

    public function get(string $param_name, $fallback) {
        return $this->getValueImpl($this->value, $param_name, $fallback, null);
    }
}