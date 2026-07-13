<?php

namespace Statsig\EvaluationTypes;

class ExperimentGroup
{
    public string $groupName;
    public string $ruleId;
    public string $idType;
    public array $returnValue;

    public function __construct(string $group_name, string $rule_id, string $id_type, array $return_value)
    {
        $this->groupName = $group_name;
        $this->ruleId = $rule_id;
        $this->idType = $id_type;
        $this->returnValue = $return_value;
    }
}
