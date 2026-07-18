defmodule Statsig.ExperimentGroup do
  @moduledoc """
  A group within an experiment, exposing its name, rule id, id type, and
  return value without requiring a user evaluation.
  """

  defstruct [
    :group_name,
    :rule_id,
    :id_type,
    :return_value
  ]

  @type t :: %__MODULE__{
          group_name: String.t(),
          rule_id: String.t(),
          id_type: String.t(),
          return_value: %{optional(String.t()) => any()}
        }
end
