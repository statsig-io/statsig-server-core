defmodule Statsig.ExperimentGroup do
  @moduledoc """
  A group within an experiment, exposing its name and return value without
  requiring a user evaluation.
  """

  defstruct [
    :group_name,
    :return_value
  ]

  @type t :: %__MODULE__{
          group_name: String.t(),
          return_value: %{optional(String.t()) => any()}
        }
end
