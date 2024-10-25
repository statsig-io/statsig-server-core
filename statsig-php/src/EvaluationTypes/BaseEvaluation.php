<?php

namespace Statsig\EvaluationTypes;

abstract class BaseEvaluation
{
    public readonly string $name;
    public readonly array $details;
    public readonly string $id_type;
    public readonly string $rule_id;
    public readonly ?string $error;

    public readonly string $__raw_result;
    public readonly array $__evaluation;

    protected function __construct(string $raw_result, $result)
    {
        $this->__raw_result = $raw_result;

        if (!is_array($result)) {
            $this->error = 'Invalid JSON input';
            $result = [];
        } else {
            $this->error = null;
        }

        $this->__evaluation = $result['__evaluation'] ?? [];

        $this->details = $result['details'] ?? [];
        $this->id_type = (string)($result['id_type'] ?? '');
        $this->name = (string)($result['name'] ?? '');
        $this->rule_id = (string)($result['rule_id'] ?? '');
    }

    protected function getValueImpl(array $value, string $param_name, $fallback, $exposure_func)
    {
        if (!array_key_exists($param_name, $value)) {
            return $fallback;
        }

        $val = $value[$param_name];

        if ($fallback !== null && gettype($val) !== gettype($fallback)) {
            return $fallback;
        }

        if ($exposure_func !== null) {
            $exposure_func($param_name);
        }

        return $val;
    }
}
