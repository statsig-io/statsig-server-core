defmodule Statsig.DataStore.ServerTest do
  use ExUnit.Case, async: true

  alias Statsig.DataStore
  alias Statsig.DataStore.BytesResponse
  alias Statsig.DataStore.Response

  @request_tag :statsig_data_store_request

  setup do
    {:ok, pid} =
      Statsig.DataStore.Server.start_link(
        __MODULE__.TestStore,
        %{values: %{"foo" => "bar"}, last_time: nil},
        bindings_module: {__MODULE__.Bindings, self()}
      )

    %{server: pid}
  end

  test "forwards get and set requests", %{server: server} do
    request = make_ref()
    send(server, {@request_tag, request, :get, "foo"})

    assert_receive {:ok, ^request, %Response{result: "bar", time: nil}}

    update = make_ref()
    send(server, {@request_tag, update, :set, {"foo", "baz", 123}})

    assert_receive {:ok, ^update, :ok}

    follow_up = make_ref()
    send(server, {@request_tag, follow_up, :get, "foo"})

    assert_receive {:ok, ^follow_up, %Response{result: "baz", time: 123}}
  end

  test "forwards get_bytes and set_bytes requests", %{server: server} do
    request = make_ref()
    send(server, {@request_tag, request, :get_bytes, "foo"})

    assert_receive {:ok, ^request, %BytesResponse{result: "bar", time: nil}}

    update = make_ref()
    send(server, {@request_tag, update, :set_bytes, {"foo", <<1, 2, 3>>, 123}})

    assert_receive {:ok, ^update, :ok}

    follow_up = make_ref()
    send(server, {@request_tag, follow_up, :get_bytes, "foo"})

    assert_receive {:ok, ^follow_up, %BytesResponse{result: <<1, 2, 3>>, time: 123}}
  end

  test "bytes requests report bytes not implemented when callback missing" do
    {:ok, pid} =
      Statsig.DataStore.Server.start_link(
        __MODULE__.BrokenStore,
        %{},
        bindings_module: {__MODULE__.Bindings, self()}
      )

    request = make_ref()
    send(pid, {@request_tag, request, :get_bytes, "foo"})

    assert_receive {:error, ^request, "BytesNotImplemented"}
  end

  test "supports polling request defaults to false when not implemented", %{server: server} do
    request = make_ref()

    send(
      server,
      {@request_tag, request, :support_polling_updates_for, "/v1/download_config_specs"}
    )

    assert_receive {:ok, ^request, false}
  end

  test "emits error when callback reports error" do
    {:ok, pid} =
      Statsig.DataStore.Server.start_link(
        __MODULE__.BrokenStore,
        %{},
        bindings_module: {__MODULE__.Bindings, self()}
      )

    request = make_ref()
    send(pid, {@request_tag, request, :get, "foo"})

    assert_receive {:error, ^request, reason}
    assert reason =~ "bad_state"
  end

  test "emits invalid return error when callback response is malformed" do
    {:ok, pid} =
      Statsig.DataStore.Server.start_link(
        __MODULE__.InvalidStore,
        %{},
        bindings_module: {__MODULE__.Bindings, self()}
      )

    request = make_ref()
    send(pid, {@request_tag, request, :get, "foo"})

    assert_receive {:error, ^request, reason}
    assert reason =~ "Invalid return"
  end

  test "support polling falls back to false when callback missing" do
    {:ok, pid} =
      Statsig.DataStore.Server.start_link(
        __MODULE__.BrokenStore,
        %{},
        bindings_module: {__MODULE__.Bindings, self()}
      )

    request = make_ref()
    send(pid, {@request_tag, request, :support_polling_updates_for, "/v2/download_config_specs"})

    assert_receive {:ok, ^request, false}
  end

  defmodule TestStore do
    @behaviour DataStore
    alias Statsig.DataStore.BytesResponse
    alias Statsig.DataStore.Response

    @impl true
    def init(state), do: {:ok, state}

    @impl true
    def handle_get(key, state) do
      {:ok,
       %Response{
         result: get_in(state, [:values, key]),
         time: state.last_time
       }, state}
    end

    @impl true
    def handle_get_bytes(key, state) do
      {:ok,
       %BytesResponse{
         result: get_in(state, [:values, key]),
         time: state.last_time
       }, state}
    end

    @impl true
    def handle_set(key, value, time, state) do
      {:ok, %{state | values: Map.put(state.values, key, value), last_time: time}}
    end

    @impl true
    def handle_set_bytes(key, value, time, state) do
      {:ok, %{state | values: Map.put(state.values, key, value), last_time: time}}
    end

    @impl true
    def handle_support_polling_updates_for(path, state) do
      {:ok, path == "/v2/download_config_specs", state}
    end
  end

  defmodule BrokenStore do
    @behaviour DataStore

    @impl true
    def init(state), do: {:ok, state}

    @impl true
    def handle_get(_key, state), do: {:error, {:bad_state, state}}
  end

  defmodule InvalidStore do
    @behaviour DataStore

    @impl true
    def init(state), do: {:ok, state}

    @impl true
    def handle_get(_key, _state), do: :invalid
  end

  defmodule Bindings do
    def data_store_reply(test_pid, ref, payload) do
      send(test_pid, {:ok, ref, payload})
      :ok
    end

    def data_store_reply_error(test_pid, ref, reason) do
      send(test_pid, {:error, ref, reason})
      :ok
    end
  end
end
