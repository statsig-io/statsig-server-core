<?php

namespace Statsig\EvaluationTypes;

class ExperimentGroup
{
    public string $groupName;
    public string $ruleId;
    public string $idType;
    public array $returnValue;

    public function __construct(array $group)
    {
        $this->groupName = (string)($group['group_name'] ?? '');
        $this->ruleId = (string)($group['rule_id'] ?? '');
        $this->idType = (string)($group['id_type'] ?? '');
        $this->returnValue = $group['return_value'] ?? [];
    }
}
