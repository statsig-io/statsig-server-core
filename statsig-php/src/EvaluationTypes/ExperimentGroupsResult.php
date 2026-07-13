<?php

namespace Statsig\EvaluationTypes;

class ExperimentGroupsResult
{
    /**
     * Null when the name does not refer to an experiment (unknown name or a
     * dynamic config); otherwise the experiment's isActive state.
     */
    public ?bool $isExperimentActive;

    /** @var ExperimentGroup[] */
    public array $groups;

    public function __construct(?bool $is_experiment_active, array $groups)
    {
        $this->isExperimentActive = $is_experiment_active;
        $this->groups = $groups;
    }
}
