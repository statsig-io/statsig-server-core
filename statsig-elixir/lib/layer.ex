# Elixir only support primitive (int, floats, bool, string) typed get, for all other more complex types
# It will be passing back as json string. Callsites need to handle by itself.
#TODO add typing

defmodule Layer do
  def get_name(layer) do
    NativeBindings.layer_get_name(layer)
  end

  def get_rule_id(layer) do
    NativeBindings.layer_get_rule_id(layer)
  end

  def get(layer, param_name, default_value) do
    NativeBindings.layer_get(layer, param_name, default_value)
  end

  def get_group_name(layer) do
    NativeBindings.layer_get_group_name(layer)
  end
end
