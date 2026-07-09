<?php

namespace Statsig\EvaluationTypes;

class Experiment extends BaseEvaluation
{
    public array $value;
    public ?string $groupName;

    public function __construct(string $raw_result)
    {
        $result = json_decode($raw_result, true);
        parent::__construct($raw_result, $result);
        if (!is_array($result)) {
            $result = [];
        }
        $this->value = $result['value'] ?? [];
        // The group-targeting getters (getExperimentByGroupName /
        // getExperimentByGroupIdAdvanced) return the camelCase ExperimentRaw
        // shape, so accept both snake_case and camelCase keys here.
        $this->groupName = $result['group_name'] ?? $result['groupName'] ?? null;
        if ($this->rule_id === '' && isset($result['ruleID'])) {
            $this->rule_id = (string)$result['ruleID'];
        }
        if ($this->id_type === '' && isset($result['idType'])) {
            $this->id_type = (string)$result['idType'];
        }
    }

    public function get(string $param_name, $fallback)
    {
        return $this->getValueImpl($this->value, $param_name, $fallback, null);
    }
}
