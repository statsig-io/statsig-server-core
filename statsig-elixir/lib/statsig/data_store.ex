defmodule Statsig.DataStore.Reference do
  @moduledoc false
  @enforce_keys [:pid]
  defstruct [:pid]

  @type t :: %__MODULE__{pid: pid()}
end

defmodule Statsig.DataStore.Response do
  @moduledoc """
  Response struct returned from `handle_get/2`.
  """

  defstruct result: nil, time: nil

  @type t :: %__MODULE__{result: nil | String.t(), time: nil | non_neg_integer()}
end

defmodule Statsig.DataStore do
  @moduledoc """
  Behaviour and helper APIs for implementing a custom Statsig data store in Elixir.

  A data store implementation is a module that implements the callbacks in this behaviour.
  Use `start_link/3` to launch the bridge process and pass the returned
  `%Statsig.DataStore.Reference{}` into `Statsig.Options`.
  """

  alias Statsig.DataStore.Reference
  alias Statsig.DataStore.Server

  @typedoc "Opaque state returned from user callbacks."
  @type state :: term()

  @callback init(init_arg :: term()) :: {:ok, state()} | {:error, term()}
  @callback handle_initialize(state()) :: {:ok, state()} | {:error, term()}
  @callback handle_shutdown(state()) :: {:ok, state()} | {:error, term()}

  @callback handle_get(String.t(), state()) ::
              {:ok, %Statsig.DataStore.Response{}, state()} | {:error, term()}

  @callback handle_set(String.t(), String.t(), non_neg_integer() | nil, state()) ::
              {:ok, state()} | {:error, term()}

  @callback handle_support_polling_updates_for(String.t(), state()) ::
              {:ok, boolean(), state()} | {:error, term()}

  @optional_callbacks handle_initialize: 1,
                      handle_shutdown: 1,
                      handle_set: 4,
                      handle_support_polling_updates_for: 2

  @doc """
  Starts a bridge process for the provided implementation module.

  Returns `{:ok, %Statsig.DataStore.Reference{}}` which can be assigned to
  `%Statsig.Options{data_store: reference}`.
  """
  @spec start_link(module(), term(), Keyword.t()) ::
          {:ok, Reference.t()} | {:error, term()}
  def start_link(module, init_arg \\ nil, opts \\ []) do
    case Server.start_link(module, init_arg, opts) do
      {:ok, pid} -> {:ok, %Reference{pid: pid}}
      other -> other
    end
  end

  @doc """
  Stops the bridge process for the given reference.
  """
  @spec stop(Reference.t(), term(), non_neg_integer()) :: :ok
  def stop(%Reference{pid: pid}, reason \\ :normal, timeout \\ 5_000) do
    GenServer.stop(pid, reason, timeout)
  end
end

defmodule Statsig.DataStore.Server do
  @moduledoc false
  use GenServer
  require Logger

  alias Statsig.DataStore.Response

  @request_tag :statsig_data_store_request
  @default_bindings Statsig.NativeBindings

  @spec start_link(module(), term(), Keyword.t()) :: GenServer.on_start()
  def start_link(module, init_arg, opts) do
    {bindings_module, gen_server_opts} =
      Keyword.put_new(opts, :bindings_module, @default_bindings)
      |> Keyword.pop(:bindings_module)

    GenServer.start_link(
      __MODULE__,
      %{module: module, module_state: nil, bindings_module: bindings_module, init_arg: init_arg},
      gen_server_opts
    )
  end

  @impl true
  def init(%{module: module, init_arg: init_arg} = state) do
    case safe_apply(module, :init, [init_arg]) do
      {:ok, module_state} ->
        {:ok,
         state
         |> Map.put(:module_state, module_state)
         |> Map.delete(:init_arg)}

      {:error, reason} ->
        {:stop, reason}

      other ->
        {:stop, {:bad_return, {module, :init, other}}}
    end
  end

  @impl true
  def handle_info(
        {@request_tag, request_ref, request_type, payload},
        %{module: module} = state
      ) do
    case dispatch(request_type, payload, state) do
      {:ok, reply_payload, new_module_state} ->
        send_reply(state.bindings_module, request_ref, reply_payload)
        {:noreply, %{state | module_state: new_module_state}}

      {:error, reason} ->
        send_error(state.bindings_module, request_ref, reason)
        {:noreply, state}

      :unknown ->
        send_error(
          state.bindings_module,
          request_ref,
          "Unknown data store request #{inspect(request_type)}"
        )

        {:noreply, state}
    end
  end

  @impl true
  def handle_info(message, state) do
    Logger.debug("Statsig.DataStore.Server received unexpected message: #{inspect(message)}")
    {:noreply, state}
  end

  defp dispatch(:initialize, _payload, state) do
    case call_without_value(state.module, :handle_initialize, 1, [state.module_state]) do
      {:ok, new_state} -> {:ok, :ok, new_state}
      :noop -> {:ok, :ok, state.module_state}
      {:error, reason} -> {:error, reason}
    end
  end

  defp dispatch(:shutdown, _payload, state) do
    case call_without_value(state.module, :handle_shutdown, 1, [state.module_state]) do
      {:ok, new_state} -> {:ok, :ok, new_state}
      :noop -> {:ok, :ok, state.module_state}
      {:error, reason} -> {:error, reason}
    end
  end

  defp dispatch(:get, key, state) do
    case maybe_call(state.module, :handle_get, 2, [key, state.module_state], :missing) do
      :missing ->
        {:error, "handle_get/2 is not implemented for #{inspect(state.module)}"}

      {:ok, %Response{} = response, new_state} ->
        {:ok, response, new_state}

      {:ok, %Response{} = response} ->
        {:ok, response, state.module_state}

      {:error, reason} ->
        {:error, reason}

      other ->
        {:error, {:bad_return, {state.module, :handle_get, other}}}
    end
  end

  defp dispatch(:set, {key, value, time}, state) do
    case call_without_value(
           state.module,
           :handle_set,
           4,
           [key, value, time, state.module_state]
         ) do
      {:ok, new_state} -> {:ok, :ok, new_state}
      :noop -> {:ok, :ok, state.module_state}
      {:error, reason} -> {:error, reason}
    end
  end

  defp dispatch(:support_polling_updates_for, path, state) do
    default = {:ok, false, state.module_state}

    case maybe_call(
           state.module,
           :handle_support_polling_updates_for,
           2,
           [path, state.module_state],
           default
         ) do
      {:ok, bool, new_state} when is_boolean(bool) ->
        {:ok, bool, new_state}

      {:ok, bool} when is_boolean(bool) ->
        {:ok, bool, state.module_state}

      {:ok, other, _new_state} ->
        {:error, {:invalid_return, other}}

      {:error, reason} ->
        {:error, reason}

      other ->
        {:error, {:bad_return, {state.module, :handle_support_polling_updates_for, other}}}
    end
  end

  defp dispatch(_type, _payload, _state), do: :unknown

  defp call_without_value(module, fun, arity, args) do
    case maybe_call(module, fun, arity, args, :ok) do
      {:ok, new_state} -> {:ok, new_state}
      {:error, reason} -> {:error, reason}
      :ok -> :noop
      other -> {:error, {:bad_return, {module, fun, other}}}
    end
  end

  defp maybe_call(module, fun, arity, args, default) do
    if function_exported?(module, fun, arity) do
      safe_apply(module, fun, args)
    else
      default
    end
  end

  defp safe_apply(module, fun, args) do
    try do
      apply(module, fun, args)
    rescue
      exception -> {:error, Exception.message(exception)}
    catch
      kind, reason -> {:error, {kind, reason}}
    end
  end

  defp send_reply({module, context}, request_ref, payload) do
    _ = module.data_store_reply(context, request_ref, payload)
  end

  defp send_reply(module, request_ref, payload) do
    _ = module.data_store_reply(request_ref, payload)
  end

  defp send_error({module, context}, request_ref, reason) do
    reason = format_reason(reason)
    _ = module.data_store_reply_error(context, request_ref, reason)
  end

  defp send_error(module, request_ref, reason) do
    reason = format_reason(reason)
    _ = module.data_store_reply_error(request_ref, reason)
  end

  defp format_reason(reason) when is_binary(reason), do: reason

  defp format_reason({:bad_return, {module, fun, value}}),
    do: "Invalid return from #{inspect(module)}.#{fun}: #{inspect(value)}"

  defp format_reason(reason), do: inspect(reason)
end
