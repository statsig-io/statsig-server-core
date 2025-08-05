defmodule Statsig.DynamicConfig do
  defstruct [
    :name,
    :value,
    :rule_id,
    :id_type
  ]

  @type t :: %__MODULE__{
          name: String.t(),
          value: String.t(),
          rule_id: String.t(),
          id_type: String.t()
        }

  def get_param_value(config, param_name) do
    config = Jason.decode!(config.value)
    case config do
      %{^param_name => value} -> value
      _ -> nil
    end
  end
end
