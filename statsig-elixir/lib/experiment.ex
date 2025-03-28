defmodule Experiment do
  @moduledoc """
  Get experiment and it's corresponding serialized values
  """

  defstruct [
    :name,
    :rule_id,
    :id_type,
    :group_name,
    :value
  ]

  @type t :: %__MODULE__{
          name: String.t(),
          rule_id: String.t(),
          id_type: String.t(),
          group_name: String.t() | nil,
          value: String.t()
        }

  def get_param_value(experiment, param_name) do
    config = Jason.decode!(experiment.value)
    case config do
      %{^param_name => value} -> value
      _ -> nil
    end
  end
end
