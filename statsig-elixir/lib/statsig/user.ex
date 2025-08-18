defmodule Statsig.User do
  defstruct [
    user_id: "",
    email: nil,
    custom: nil,
    custom_ids: nil,
    private_attributes: nil,
    ip: nil,
    user_agent: nil,
    country: nil,
    locale: nil,
    app_version: nil,
  ]

  def new(attrs) when is_map(attrs) do
    %__MODULE__{}
    |> struct(attrs)
    |> validate_presence_of_id()
  end

  defp validate_presence_of_id(%__MODULE__{user_id: nil, custom_ids: nil}) do
    raise ArgumentError,
          "Either `user_id` or `custom_ids` must be set"
  end

  defp validate_presence_of_id(struct), do: struct

  @type custom_value :: String.t() | number() | boolean() | nil
  @type custom_attributes :: %{String.t() => custom_value()}

  @type t :: %__MODULE__{
    user_id: String.t() | nil,
    custom_ids: %{optional(String.t()) => String.t()} | nil,
    email: String.t() | nil,
    ip: String.t() | nil,
    user_agent: String.t() | nil,
    country: String.t() | nil,
    locale: String.t() | nil,
    app_version: String.t() | nil,
    custom: custom_attributes() | nil,
    private_attributes: custom_attributes() | nil,
  }

end
