# Elixir only support primitive (int, floats, bool, string) typed get, for all other more complex types
# It will be passing back as json string. Callsites need to handle by itself.
# TODO add typing

defmodule Layer do
  def get_name(layer) do
    try do
      NativeBindings.layer_get_name(layer)
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_rule_id(layer) do
    try do
      NativeBindings.layer_get_rule_id(layer)
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get(layer, param_name, default_value) do
    try do
      NativeBindings.layer_get(layer, param_name, default_value)
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_group_name(layer) do
    try do
      NativeBindings.layer_get_group_name(layer)
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end
end
