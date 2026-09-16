defmodule Statsig.ObservabilityClient.Reference do
  @moduledoc false
  @enforce_keys [:pid]
  defstruct [:pid, high_cardinality_tags: nil]

  @type t :: %__MODULE__{pid: pid(), high_cardinality_tags: [String.t()] | nil}
end

defmodule Statsig.ObservabilityClient do
  @moduledoc """
  Behaviour and helper APIs for forwarding Statsig SDK metrics to an
  observability backend (Datadog, Prometheus, etc.) from Elixir.

  An implementation is a module that implements the callbacks in this
  behaviour. Use `start_link/3` to launch the bridge process and pass the
  returned `%Statsig.ObservabilityClient.Reference{}` into `Statsig.Options`.

  Callbacks are invoked fire-and-forget by the core: the SDK does not wait for
  a reply, and there is no backpressure. Callbacks should be non-blocking — a
  slow callback (e.g. a synchronous HTTP push inside `handle_increment`) lets
  the bridge process mailbox grow unbounded. Hand slow work off to another
  process instead.

  The core's high-cardinality tags (`lcut` and `prev_lcut`) are dropped before
  the metric callbacks are invoked unless allowlisted. The synchronous
  `should_enable_high_cardinality_for_this_tag` callback cannot round-trip over
  the fire-and-forget bridge, so the allowlist is configured statically via the
  `:high_cardinality_tags` option to `start_link/3`:

      {:ok, ref} =
        Statsig.ObservabilityClient.start_link(MyClient, nil,
          high_cardinality_tags: ["lcut"]
        )
  """

  alias Statsig.ObservabilityClient.Reference
  alias Statsig.ObservabilityClient.Server

  @typedoc "Opaque state returned from user callbacks."
  @type state :: term()

  @typedoc "Metric tags, or nil when the core supplies none."
  @type tags :: %{optional(String.t()) => String.t()} | nil

  @callback init(init_arg :: term()) :: {:ok, state()} | {:error, term()}
  @callback handle_init(state()) :: {:ok, state()} | :ok
  @callback handle_increment(String.t(), number(), tags(), state()) :: {:ok, state()} | :ok
  @callback handle_gauge(String.t(), number(), tags(), state()) :: {:ok, state()} | :ok
  @callback handle_dist(String.t(), number(), tags(), state()) :: {:ok, state()} | :ok
  @callback handle_error(String.t(), String.t(), state()) :: {:ok, state()} | :ok

  @doc """
  Invoked when the bridge process shuts down gracefully (e.g. via `stop/3`).

  A last chance for implementations that buffer metrics in their state to
  flush them. Subject to the usual `c:GenServer.terminate/2` caveats: not
  guaranteed to run on abnormal exits.
  """
  @callback handle_terminate(reason :: term(), state()) :: term()

  @optional_callbacks handle_init: 1,
                      handle_increment: 4,
                      handle_gauge: 4,
                      handle_dist: 4,
                      handle_error: 3,
                      handle_terminate: 2

  @doc """
  Starts a bridge process for the provided implementation module.

  Returns `{:ok, %Statsig.ObservabilityClient.Reference{}}` which can be
  assigned to `%Statsig.Options{observability_client: reference}`.

  ## Options

    * `:high_cardinality_tags` - which of the core's high-cardinality tags to
      keep on metrics. The core currently consults this only for `"lcut"` and
      `"prev_lcut"`; other values are a no-op, and tags outside that set are
      always kept regardless of this option. Defaults to `nil` (core default:
      `lcut`/`prev_lcut` are dropped).

  Remaining options are passed to `GenServer.start_link/3`.
  """
  @spec start_link(module(), term(), Keyword.t()) ::
          {:ok, Reference.t()} | :ignore | {:error, term()}
  def start_link(module, init_arg \\ nil, opts \\ []) do
    {high_cardinality_tags, opts} = Keyword.pop(opts, :high_cardinality_tags)

    case Server.start_link(module, init_arg, opts) do
      {:ok, pid} -> {:ok, %Reference{pid: pid, high_cardinality_tags: high_cardinality_tags}}
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

defmodule Statsig.ObservabilityClient.Server do
  @moduledoc false
  use GenServer
  require Logger

  @request_tag :statsig_observability_client_request

  @spec start_link(module(), term(), Keyword.t()) :: GenServer.on_start()
  def start_link(module, init_arg, opts) do
    GenServer.start_link(
      __MODULE__,
      %{module: module, module_state: nil, init_arg: init_arg},
      opts
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
  def handle_info({@request_tag, action, payload}, state) do
    {:noreply, %{state | module_state: dispatch(action, payload, state)}}
  end

  @impl true
  def handle_info(message, state) do
    Logger.debug(
      "Statsig.ObservabilityClient.Server received unexpected message: #{inspect(message)}"
    )

    {:noreply, state}
  end

  @impl true
  def terminate(reason, %{module: module, module_state: module_state}) do
    if function_exported?(module, :handle_terminate, 2) do
      safe_apply(module, :handle_terminate, [reason, module_state])
    end

    :ok
  end

  defp dispatch(:init, _payload, state) do
    invoke(state, :handle_init, 1, [state.module_state])
  end

  defp dispatch(:increment, {metric, value, tags}, state) do
    invoke(state, :handle_increment, 4, [metric, value, tags, state.module_state])
  end

  defp dispatch(:gauge, {metric, value, tags}, state) do
    invoke(state, :handle_gauge, 4, [metric, value, tags, state.module_state])
  end

  defp dispatch(:dist, {metric, value, tags}, state) do
    invoke(state, :handle_dist, 4, [metric, value, tags, state.module_state])
  end

  defp dispatch(:error, {tag, error}, state) do
    invoke(state, :handle_error, 3, [tag, error, state.module_state])
  end

  defp dispatch(other, _payload, state) do
    Logger.debug("Statsig.ObservabilityClient.Server unknown request #{inspect(other)}")
    state.module_state
  end

  defp invoke(state, fun, arity, args) do
    if function_exported?(state.module, fun, arity) do
      case safe_apply(state.module, fun, args) do
        {:ok, new_state} ->
          new_state

        :ok ->
          state.module_state

        {:error, reason} ->
          Logger.warning("Statsig.ObservabilityClient #{fun} failed: #{inspect(reason)}")
          state.module_state

        other ->
          Logger.warning(
            "Statsig.ObservabilityClient #{fun} returned unexpected value: #{inspect(other)}"
          )

          state.module_state
      end
    else
      state.module_state
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
end
