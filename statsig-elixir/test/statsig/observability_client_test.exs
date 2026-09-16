defmodule Statsig.ObservabilityClientTest do
  @moduledoc """
  Exercises the native Rust bridge end-to-end: the test NIF drives the
  ObservabilityClient trait implementation, which messages the bridge process,
  which dispatches to the user callback module.
  """
  use ExUnit.Case, async: false

  alias Statsig.NativeBindings
  alias Statsig.ObservabilityClient

  defmodule TestClient do
    @behaviour ObservabilityClient

    @impl true
    def init(%{test_pid: pid}), do: {:ok, %{test_pid: pid}}

    @impl true
    def handle_init(%{test_pid: pid} = state) do
      send(pid, {:obs, :init})
      {:ok, state}
    end

    @impl true
    def handle_increment(metric, value, tags, %{test_pid: pid} = state) do
      send(pid, {:obs, :increment, metric, value, tags})
      {:ok, state}
    end

    @impl true
    def handle_gauge(metric, value, tags, %{test_pid: pid} = state) do
      send(pid, {:obs, :gauge, metric, value, tags})
      {:ok, state}
    end

    @impl true
    def handle_dist(metric, value, tags, %{test_pid: pid} = state) do
      send(pid, {:obs, :dist, metric, value, tags})
      {:ok, state}
    end

    @impl true
    def handle_error(tag, error, %{test_pid: pid} = state) do
      send(pid, {:obs, :error, tag, error})
      {:ok, state}
    end
  end

  setup do
    {:ok, ref} = ObservabilityClient.start_link(TestClient, %{test_pid: self()})

    on_exit(fn ->
      if Process.alive?(ref.pid), do: ObservabilityClient.stop(ref)
    end)

    %{ref: ref}
  end

  test "drives init through the native bridge", %{ref: ref} do
    NativeBindings.__internal__test_observability_client(ref, "init", "", 0.0, nil)
    assert_receive {:obs, :init}
  end

  test "drives increment, gauge, and dist through the native bridge", %{ref: ref} do
    NativeBindings.__internal__test_observability_client(
      ref,
      "increment",
      "metric_inc",
      1.0,
      %{"k" => "v"}
    )

    assert_receive {:obs, :increment, "metric_inc", 1.0, %{"k" => "v"}}

    NativeBindings.__internal__test_observability_client(
      ref,
      "gauge",
      "metric_gauge",
      2.0,
      %{"k" => "v"}
    )

    assert_receive {:obs, :gauge, "metric_gauge", 2.0, %{"k" => "v"}}

    NativeBindings.__internal__test_observability_client(ref, "dist", "metric_dist", 3.0, nil)
    assert_receive {:obs, :dist, "metric_dist", 3.0, nil}
  end

  test "drives error through the native bridge", %{ref: ref} do
    NativeBindings.__internal__test_observability_client(ref, "error", "my_tag", 0.0, %{
      "error" => "boom"
    })

    assert_receive {:obs, :error, "my_tag", "boom"}
  end

  # Untested seam: these assert the callback's answer one layer below where it
  # takes effect. The core applies it inside OpsStatsEventObserver::handle_event
  # (observability_client_adapter.rs), which the test NIF bypasses — so nothing
  # here proves an allowlisted `lcut` tag survives to handle_increment, and only
  # init is covered from a real Statsig instance (metric emission timing is
  # core-internal and would make such a test flaky).
  test "answers should_enable_high_cardinality_for_this_tag from the static allowlist" do
    {:ok, ref} =
      ObservabilityClient.start_link(TestClient, %{test_pid: self()},
        high_cardinality_tags: ["lcut"]
      )

    on_exit(fn -> if Process.alive?(ref.pid), do: ObservabilityClient.stop(ref) end)

    assert NativeBindings.__internal__test_observability_client(
             ref,
             "should_enable_high_cardinality_for_this_tag",
             "lcut",
             0.0,
             nil
           ) == true

    assert NativeBindings.__internal__test_observability_client(
             ref,
             "should_enable_high_cardinality_for_this_tag",
             "prev_lcut",
             0.0,
             nil
           ) == false
  end

  test "should_enable_high_cardinality_for_this_tag defers to the core default when unconfigured",
       %{ref: ref} do
    assert NativeBindings.__internal__test_observability_client(
             ref,
             "should_enable_high_cardinality_for_this_tag",
             "lcut",
             0.0,
             nil
           ) == nil
  end

  # Exercises the real option wiring in statsig_nfi.rs: the strong Arc is built
  # from the reference, downgraded into StatsigOptions, and Statsig::new invokes
  # the client's init() synchronously via setup_ops_stats — so receiving
  # {:obs, :init} proves the native path end-to-end.
  test "Statsig.start_link invokes init through the registered observability client", %{ref: ref} do
    options = %Statsig.Options{
      observability_client: ref,
      disable_network: true,
      disable_all_logging: true
    }

    assert {:ok, _pid} = Statsig.start_link("secret-key", options)

    on_exit(fn ->
      Statsig.shutdown()

      if pid = Process.whereis(Statsig) do
        Process.exit(pid, :normal)
      end
    end)

    assert_receive {:obs, :init}
  end
end
