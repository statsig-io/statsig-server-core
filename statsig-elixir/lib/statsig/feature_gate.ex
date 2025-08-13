defmodule Statsig.FeatureGate do
  defstruct [
    :name,
    :value,
    :rule_id,
    :id_type
  ]

  @type t :: %__MODULE__{
          name: String.t(),
          value: boolean(),
          rule_id: String.t(),
          id_type: String.t()
        }
end
