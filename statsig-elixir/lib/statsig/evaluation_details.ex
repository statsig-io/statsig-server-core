defmodule Statsig.EvaluationDetails do
  defstruct [
    :reason,
    :lcut,
    :received_at,
    :version
  ]

  @type t :: %__MODULE__{
          reason: String.t(),
          lcut: non_neg_integer() | nil,
          received_at: non_neg_integer() | nil,
          version: non_neg_integer() | nil
        }
end
