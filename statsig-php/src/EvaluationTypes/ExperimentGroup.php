<?php

namespace Statsig\EvaluationTypes;

class ExperimentGroup
{
    public string $groupName;
    public array $returnValue;

    public function __construct(string $group_name, array $return_value)
    {
        $this->groupName = $group_name;
        $this->returnValue = $return_value;
    }
}
