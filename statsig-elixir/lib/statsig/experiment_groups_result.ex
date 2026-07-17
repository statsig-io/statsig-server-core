defmodule Statsig.ExperimentGroupsResult do
  @moduledoc """
  The groups of an experiment along with its active state, exposed without
  requiring a user evaluation.

  `is_experiment_active` is `nil` when the name does not refer to an experiment
  (unknown name or a non-experiment entity like a dynamic config or autotune);
  otherwise it reflects the experiment's `isActive` state.
  """

  defstruct [
    :is_experiment_active,
    :groups
  ]

  @type t :: %__MODULE__{
          is_experiment_active: boolean() | nil,
          groups: [Statsig.ExperimentGroup.t()]
        }
end
