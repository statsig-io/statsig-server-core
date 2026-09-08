defmodule Statsig.ObservabilityClient.ServerTest do
  use ExUnit.Case, async: true

  alias Statsig.ObservabilityClient

  @request_tag :statsig_observability_client_request

  setup do
    parent = self()

    {:ok, pid} =
      ObservabilityClient.Server.start_link(__MODULE__.TestClient, %{test_pid: parent}, [])

    %{server: pid}
  end

  test "forwards init", %{server: server} do
    send(server, {@request_tag, :init, :no_payload})
    assert_receive {:callback, :init}
  end

  test "forwards increment, gauge, and dist", %{server: server} do
    send(server, {@request_tag, :increment, {"metric_inc", 1.0, %{"k" => "v"}}})
    assert_receive {:callback, {:increment, "metric_inc", 1.0, %{"k" => "v"}}}

    send(server, {@request_tag, :gauge, {"metric_gauge", 2.0, nil}})
    assert_receive {:callback, {:gauge, "metric_gauge", 2.0, nil}}

    send(server, {@request_tag, :dist, {"metric_dist", 3.0, %{"k" => "v"}}})
    assert_receive {:callback, {:dist, "metric_dist", 3.0, %{"k" => "v"}}}
  end

  test "forwards error", %{server: server} do
    send(server, {@request_tag, :error, {"my_tag", "boom"}})
    assert_receive {:callback, {:error, "my_tag", "boom"}}
  end

  test "threads module state across callbacks", %{server: server} do
    # The callback reports the running count it derives from its own state, so
    # this asserts state threading behaviorally rather than via :sys.get_state.
    send(server, {@request_tag, :increment, {"m", 1.0, nil}})
    assert_receive {:callback, {:count, 1}}
    send(server, {@request_tag, :increment, {"m", 1.0, nil}})
    assert_receive {:callback, {:count, 2}}
  end

  test "invokes handle_terminate on graceful stop", %{server: server} do
    GenServer.stop(server, :normal)
    assert_receive {:callback, {:terminate, :normal}}
  end

  test "ignores unimplemented optional callbacks without crashing" do
    {:ok, pid} = ObservabilityClient.Server.start_link(__MODULE__.MinimalClient, %{}, [])

    send(pid, {@request_tag, :increment, {"m", 1.0, nil}})
    # Round-trip a state read to ensure the message was processed.
    assert %{module_state: %{}} = :sys.get_state(pid)
    assert Process.alive?(pid)
  end

  test "survives a raising callback", %{server: server} do
    send(server, {@request_tag, :error, {"raise", "boom"}})
    # A raise is caught; the server keeps running and continues to process work.
    send(server, {@request_tag, :init, :no_payload})
    assert_receive {:callback, :init}
    assert Process.alive?(server)
  end

  test "ignores unknown request types", %{server: server} do
    send(server, {@request_tag, :not_a_real_action, {"m", 1.0, nil}})
    send(server, {@request_tag, :init, :no_payload})
    assert_receive {:callback, :init}
    assert Process.alive?(server)
  end

  defmodule TestClient do
    @behaviour ObservabilityClient

    @impl true
    def init(state), do: {:ok, Map.put(state, :count, 0)}

    @impl true
    def handle_init(state) do
      notify(state, :init)
      {:ok, state}
    end

    @impl true
    def handle_increment(metric, value, tags, state) do
      notify(state, {:increment, metric, value, tags})
      new_state = Map.update(state, :count, 1, &(&1 + 1))
      notify(new_state, {:count, new_state.count})
      {:ok, new_state}
    end

    @impl true
    def handle_gauge(metric, value, tags, state) do
      notify(state, {:gauge, metric, value, tags})
      :ok
    end

    @impl true
    def handle_dist(metric, value, tags, state) do
      notify(state, {:dist, metric, value, tags})
      :ok
    end

    @impl true
    def handle_error("raise", _error, _state), do: raise("boom from callback")

    def handle_error(tag, error, state) do
      notify(state, {:error, tag, error})
      :ok
    end

    @impl true
    def handle_terminate(reason, state) do
      notify(state, {:terminate, reason})
    end

    defp notify(%{test_pid: pid}, message) when is_pid(pid), do: send(pid, {:callback, message})
    defp notify(_, _message), do: :ok
  end

  defmodule MinimalClient do
    @behaviour ObservabilityClient

    @impl true
    def init(state), do: {:ok, state}
  end
end
