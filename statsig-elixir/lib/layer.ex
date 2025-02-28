defmodule Layer do
  @moduledoc """
  Functions to get values, metadata e.g. group_name for layer
  Layer object is not a struct, instead it's a reference to a layer struct defined in statsig-core (Binary library)
  """
  def get_name(layer) do
    try do
      {:ok, NativeBindings.layer_get_name(layer)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_rule_id(layer) do
    try do
      {:ok, NativeBindings.layer_get_rule_id(layer)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  @doc """
  Get parameter within a layer
  ## Parameters:
  - `layer` Layer reference returned from Statsig.get_layer
  - `param_name` (String.t()): Parameter name you want to get.
  - `default_value` (String.t() | number() | boolean): Default value if no related param is found. If value is a more complex type, e.g. array, object, it returns json serialized value

  ## Returns:
  - `value` (String.t() | number() | boolean): If the function runs succesfully. It returns
  - `{:error, :_}`: If any exception happens when execute this function
  """
  @spec get(any, String.t(), String.t() | boolean | number) :: String.t() | boolean | number
  def get(layer, param_name, default_value) do
    try do
      {:ok, NativeBindings.layer_get(layer, param_name, default_value)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_group_name(layer) do
    try do
      {:ok, NativeBindings.layer_get_group_name(layer)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end
end
