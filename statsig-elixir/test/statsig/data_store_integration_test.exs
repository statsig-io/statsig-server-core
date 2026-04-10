defmodule Statsig.DataStore.IntegrationTest do
  use ExUnit.Case, async: false

  alias Statsig.DataStore
  alias Statsig.DataStore.BytesResponse
  alias Statsig.DataStore.Response
  alias Statsig.User

  defmodule TestDataStore do
    @behaviour DataStore
    @eval_proj_path Path.expand("../../../statsig-rust/tests/data/perf_proj_dcs.json", __DIR__)
    @eval_proj_data File.read!(@eval_proj_path)

    @impl true
    def init(opts) do
      notify(opts, :init)

      state =
        opts
        |> Map.put_new(:get_count, 0)
        |> Map.put_new(:set_count, 0)
        |> Map.put_new(:enable_polling_count, 0)
        |> Map.put_new(:return_valid_response, true)

      {:ok, state}
    end

    @impl true
    def handle_initialize(state) do
      notify(state, :handle_initialize)
      {:ok, state}
    end

    @impl true
    def handle_shutdown(state) do
      notify(state, :handle_shutdown)
      {:ok, state}
    end

    @impl true
    def handle_get(_key, state) do
      notify(state, :handle_get)
      state = Map.update!(state, :get_count, &(&1 + 1))

      result =
        if Map.get(state, :return_valid_response, true) do
          @eval_proj_data
        else
          ""
        end

      response = %Response{
        result: result,
        time: 23
      }

      {:ok, response, state}
    end

    @impl true
    def handle_get_bytes(key, state) do
      notify(state, :handle_get_bytes)

      if String.contains?(key, "|statsig-br|") do
        {:ok, %BytesResponse{result: nil, time: nil}, state}
      else
        state = Map.update!(state, :get_count, &(&1 + 1))

        result =
          if Map.get(state, :return_valid_response, true) do
            @eval_proj_data
          else
            ""
          end

        {:ok, %BytesResponse{result: result, time: 23}, state}
      end
    end

    @impl true
    def handle_set(_key, _value, _time, state) do
      notify(state, :handle_set)
      state = Map.update!(state, :set_count, &(&1 + 1))
      {:ok, state}
    end

    @impl true
    def handle_set_bytes(_key, _value, _time, state) do
      notify(state, :handle_set_bytes)
      state = Map.update!(state, :set_count, &(&1 + 1))
      {:ok, state}
    end

    @impl true
    def handle_support_polling_updates_for(_path, state) do
      notify(state, :handle_support_polling_updates_for)
      state = Map.update!(state, :enable_polling_count, &(&1 + 1))
      {:ok, false, state}
    end

    defp notify(%{test_pid: pid}, message) when is_pid(pid) do
      send(pid, {:callback, message})
    end

    defp notify(_, _message), do: :ok

    def counts(%Statsig.DataStore.Reference{pid: pid}) do
      %{module_state: module_state} = :sys.get_state(pid)

      %{
        get: Map.get(module_state, :get_count, 0),
        set: Map.get(module_state, :set_count, 0),
        enable_polling: Map.get(module_state, :enable_polling_count, 0)
      }
    end

    def eval_proj_data, do: @eval_proj_data
  end

  defmodule MockScrapi do
    defstruct [:pid, :socket, :specs_url]

    def start_link(response_body, test_pid) do
      {:ok, socket} =
        :gen_tcp.listen(0, [
          :binary,
          active: false,
          packet: :raw,
          reuseaddr: true,
          ip: {127, 0, 0, 1}
        ])

      {:ok, {{127, 0, 0, 1}, port}} = :inet.sockname(socket)
      pid = spawn_link(fn -> accept_loop(socket, response_body, test_pid) end)

      {:ok,
       %__MODULE__{
         pid: pid,
         socket: socket,
         specs_url: "http://127.0.0.1:#{port}/v2/download_config_specs"
       }}
    end

    def specs_url(%__MODULE__{specs_url: specs_url}), do: specs_url

    def stop(%__MODULE__{pid: pid, socket: socket}) do
      :gen_tcp.close(socket)

      if Process.alive?(pid) do
        Process.unlink(pid)
        Process.exit(pid, :shutdown)
      end
    end

    defp accept_loop(socket, response_body, test_pid) do
      case :gen_tcp.accept(socket) do
        {:ok, client} ->
          handle_client(client, response_body, test_pid)
          accept_loop(socket, response_body, test_pid)

        {:error, _reason} ->
          :ok
      end
    end

    defp handle_client(client, response_body, test_pid) do
      case :gen_tcp.recv(client, 0, 5_000) do
        {:ok, request} ->
          request_line = request |> String.split("\r\n", parts: 2) |> hd()
          send(test_pid, {:mock_scrapi_request, request_line})
          :gen_tcp.send(client, http_response(response_body))

        {:error, _reason} ->
          :ok
      end

      :gen_tcp.close(client)
    end

    defp http_response(response_body) do
      [
        "HTTP/1.1 200 OK\r\n",
        "content-type: application/json\r\n",
        "content-length: #{byte_size(response_body)}\r\n",
        "connection: close\r\n",
        "\r\n",
        response_body
      ]
    end
  end

  @tag capture_log: false
  test "Statsig starts and initializes with a custom data store" do
    parent = self()

    {:ok, data_store_ref} = DataStore.start_link(TestDataStore, %{test_pid: parent})

    on_exit(fn ->
      if Process.alive?(data_store_ref.pid) do
        DataStore.stop(data_store_ref)
      end
    end)

    assert_receive {:callback, :init}

    options = %Statsig.Options{
      data_store: data_store_ref,
      disable_all_logging: true,
      specs_sync_interval_ms: 1000,
      output_log_level: "debug",
      spec_adapter_configs: [
        %Statsig.SpecAdapterConfig{
          adapter_type: "data_store",
          init_timeout_ms: 3_000
        }
      ]
    }

    sdk_key = System.get_env("test_api_key")

    assert {:ok, _pid} = Statsig.start_link(sdk_key, options)

    on_exit(fn ->
      Statsig.shutdown()

      if pid = Process.whereis(Statsig) do
        Process.exit(pid, :normal)
      end
    end)

    assert Process.alive?(Process.whereis(Statsig))

    Statsig.initialize()

    user = User.new(%{user_id: "integration-user"})
    Process.sleep(4000)
    assert {:ok, true} = Statsig.check_gate("test_public", user)
    assert {:ok, gate} = Statsig.get_feature_gate("test_public", user)
    assert gate.value
    counts = TestDataStore.counts(data_store_ref)
    assert counts.get == 1
    assert counts.enable_polling == 1
  end

  @tag capture_log: false
  test "spec adapter config prioritizes the data store adapter" do
    parent = self()

    {:ok, data_store_ref} =
      DataStore.start_link(TestDataStore, %{test_pid: parent, return_valid_response: false})

    on_exit(fn ->
      if Process.alive?(data_store_ref.pid) do
        DataStore.stop(data_store_ref)
      end
    end)

    assert_receive {:callback, :init}
    {:ok, mock_scrapi} = MockScrapi.start_link(TestDataStore.eval_proj_data(), parent)

    on_exit(fn ->
      MockScrapi.stop(mock_scrapi)
    end)

    options = %Statsig.Options{
      data_store: data_store_ref,
      disable_all_logging: true,
      specs_sync_interval_ms: 1000,
      spec_adapter_configs: [
        %Statsig.SpecAdapterConfig{
          adapter_type: "data_store",
          init_timeout_ms: 3_000
        },
        %Statsig.SpecAdapterConfig{
          adapter_type: "network_http",
          specs_url: MockScrapi.specs_url(mock_scrapi)
        }
      ]
    }

    sdk_key = System.get_env("test_api_key")
    assert {:ok, _pid} = Statsig.start_link(sdk_key, options)

    on_exit(fn ->
      Statsig.shutdown()

      if pid = Process.whereis(Statsig) do
        Process.exit(pid, :normal)
      end
    end)

    Statsig.initialize()
    assert_receive {:mock_scrapi_request, request_line}, 1_000
    assert String.contains?(request_line, "/v2/download_config_specs/")

    user = User.new(%{user_id: "spec-adapter-user"})
    Process.sleep(4_000)
    assert {:ok, true} = Statsig.check_gate("test_public", user)

    counts = TestDataStore.counts(data_store_ref)
    assert counts.get == 1
  end
end
