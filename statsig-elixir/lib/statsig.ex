defmodule Statsig do
  use GenServer

  alias Statsig.NativeBindings

  def start_link(sdk_key, options) do
    GenServer.start_link(__MODULE__, {sdk_key, options}, name: __MODULE__)
  end

  def init({sdk_key, statsig_options}) do
    try do
      instance = NativeBindings.new(sdk_key, statsig_options)
      {:ok, instance}
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def handle_call(:get_instance, _from, state) do
    {:reply, state, state}
  end

  def get_statsig_instance do
    GenServer.call(__MODULE__, :get_instance)
  end

  def initialize() do
    try do
      instance = get_statsig_instance()
      NativeBindings.initialize(instance)
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:stop, reason}
      exception -> {:stop, Exception.message(exception)}
    end
  end

  def check_gate(gate_name, statsig_user) do
    try do
      instance = get_statsig_instance()

      {:ok, NativeBindings.check_gate(instance, gate_name, statsig_user)}
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_feature_gate(gate_name, statsig_user) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_feature_gate(instance, gate_name, statsig_user) do
        {:error, e} -> {:error, e}
        gate -> {:ok, gate}
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_config(config_name, statsig_user) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_config(instance, config_name, statsig_user) do
        {:error, e} -> {:error, e}
        config -> {:ok, config}
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_experiment(experiment_name, statsig_user) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_experiment(instance, experiment_name, statsig_user) do
        {:error, e} -> {:error, e}
        exp -> {:ok, exp}
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_layer(layer_name, statsig_user) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_layer(instance, layer_name, statsig_user) do
        {:error, e} -> {:error, e}
        layer -> {:ok, layer}
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  @spec log_event(%Statsig.User{}, String.t(), String.t() | number(), %{String.t() => String.t()}) ::
          any()
  def log_event(statsig_user, event_name, value, metadata) do
    try do
      instance = get_statsig_instance()

      case value do
        value when is_binary(value) ->
          NativeBindings.log_event(instance, statsig_user, event_name, value, metadata)

        value when is_number(value) ->
          NativeBindings.log_event_with_number(
            instance,
            statsig_user,
            event_name,
            value,
            metadata
          )

        _ ->
          {:error, :invalid_value}
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def flush() do
    try do
      instance = get_statsig_instance()
      NativeBindings.flush(instance)
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def shutdown() do
    try do
      instance = get_statsig_instance()
      NativeBindings.shutdown(instance)
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end
end
