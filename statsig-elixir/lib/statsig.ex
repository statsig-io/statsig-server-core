defmodule Statsig do
  use GenServer

  alias Statsig.NativeBindings

  def start_link(sdk_key, options) do
    GenServer.start_link(__MODULE__, {sdk_key, options}, name: __MODULE__)
  end

  def init({sdk_key, statsig_options}) do
    try do
      instance = NativeBindings.new(sdk_key, statsig_options, get_system_info())
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

  def check_gate(gate_name, statsig_user, options \\ nil) do
    try do
      instance = get_statsig_instance()

      {:ok, NativeBindings.check_gate(instance, gate_name, statsig_user, options)}
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_feature_gate(gate_name, statsig_user, options \\ nil) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_feature_gate(instance, gate_name, statsig_user, options) do
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

  def get_dynamic_config(config_name, statsig_user, options \\ nil) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_dynamic_config(instance, config_name, statsig_user, options) do
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

  def get_experiment(experiment_name, statsig_user, options \\ nil) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_experiment(instance, experiment_name, statsig_user, options) do
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

  def get_experiment_by_group_name(experiment_name, group_name) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_experiment_by_group_name(instance, experiment_name, group_name) do
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

  def get_experiment_by_group_id_advanced(experiment_name, group_id) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_experiment_by_group_id_advanced(instance, experiment_name, group_id) do
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

  def override_experiment_by_group_name(experiment_name, group_name, id \\ nil) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.override_experiment_by_group_name(
             instance,
             experiment_name,
             group_name,
             id
           ) do
        {:error, e} -> {:error, e}
        _ -> :ok
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  @doc """
  Returns the experiment's active state and the group name, rule id, id type, and
  return value for each of its groups, without requiring a user evaluation.

  Returns `{:ok, %Statsig.ExperimentGroupsResult{}}` where `is_experiment_active` is
  `nil` if the name does not refer to an experiment (unknown name or a dynamic config),
  and otherwise reflects the experiment's `isActive` state; `groups` contains the
  experiment's groups regardless of that state. Rules that are not experiment groups
  (e.g. holdout or sizing rules) are excluded.
  """
  def get_experiment_groups(experiment_name) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_experiment_groups(instance, experiment_name) do
        {:error, e} -> {:error, e}
        result -> {:ok, result}
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_layer(layer_name, statsig_user, options \\ nil) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_layer(instance, layer_name, statsig_user, options) do
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

  def get_client_init_response_as_string(statsig_user, options \\ nil) do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_client_init_response_as_string(instance, statsig_user, options) do
        {:error, e} -> {:error, e}
        response -> {:ok, response}
      end
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      :exit, reason -> {:error, {:exit, reason}}
      exception -> {:error, Exception.message(exception)}
    end
  end

  def get_autotune_list() do
    try do
      instance = get_statsig_instance()

      case NativeBindings.get_autotune_list(instance) do
        {:error, e} -> {:error, e}
        list -> {:ok, list}
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
        value when is_binary(value) or is_nil(value) ->
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

  def get_system_info do
    try do
      %{
        "os" => :os.type() |> elem(0) |> Atom.to_string(),
        "arch" => :erlang.system_info(:system_architecture) |> List.to_string(),
        "language_version" => System.version()
      }
    rescue
      _ ->
        %{
          "os" => "unknown",
          "arch" => "unknown",
          "language_version" => "unknown"
        }
    catch
      _, _ ->
        %{
          "os" => "unknown",
          "arch" => "unknown",
          "language_version" => "unknown"
        }
    end
  end
end
