<?php

namespace Statsig\EvaluationTypes;

class ExperimentGroupsResult
{
    /**
     * Null when the name does not refer to an experiment (unknown name or
     * a non-experiment entity like a dynamic config or autotune);
     * otherwise the experiment's isActive state.
     */
    public ?bool $isExperimentActive;

    /** @var ExperimentGroup[] */
    public array $groups;

    public function __construct(string $raw_result)
    {
        $result = json_decode($raw_result, true);
        if (!is_array($result)) {
            $result = [];
        }

        $this->isExperimentActive = $result['is_experiment_active'] ?? null;
        $this->groups = array_map(
            fn ($group) => new ExperimentGroup($group),
            $result['groups'] ?? []
        );
    }
}
