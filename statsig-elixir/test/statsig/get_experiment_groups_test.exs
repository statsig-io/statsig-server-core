defmodule Statsig.GetExperimentGroupsTest do
  use ExUnit.Case, async: false

  alias Statsig.GetExperimentGroupsTest.MockScrapi
  alias Statsig.Options

  @eval_proj_path Path.expand("../../../statsig-rust/tests/data/eval_proj_dcs.json", __DIR__)
  @eval_proj_data File.read!(@eval_proj_path)

  setup do
    {:ok, mock} = MockScrapi.start_link(@eval_proj_data)

    options = %Options{
      specs_url: MockScrapi.specs_url(mock),
      disable_all_logging: true,
      output_log_level: "warn"
    }

    {:ok, _pid} = Statsig.start_link("secret-key", options)
    Statsig.initialize()

    on_exit(fn ->
      Statsig.shutdown()

      if pid = Process.whereis(Statsig) do
        Process.exit(pid, :normal)
      end

      MockScrapi.stop(mock)
    end)

    :ok
  end

  test "returns the groups for a known experiment" do
    {:ok, groups} = Statsig.get_experiment_groups("test_experiment_no_targeting")

    groups_by_name =
      Map.new(groups, fn group -> {group.group_name, group.return_value} end)

    # Only the experiment group rules are returned (the layerAssignment rule is excluded).
    assert Enum.sort(Map.keys(groups_by_name)) == ["Control", "Test", "Test2"]
    assert groups_by_name["Control"] == %{"value" => "control"}
    assert groups_by_name["Test"] == %{"value" => "test_1"}
    assert groups_by_name["Test2"] == %{"value" => "test_2"}
  end

  test "returns ExperimentGroup structs" do
    {:ok, groups} = Statsig.get_experiment_groups("test_experiment_no_targeting")

    assert Enum.all?(groups, &match?(%Statsig.ExperimentGroup{}, &1))
  end

  test "returns an empty list for an unknown experiment" do
    assert {:ok, []} = Statsig.get_experiment_groups("nonexistent_experiment")
  end

  test "returns an empty list for a dynamic config" do
    assert {:ok, []} = Statsig.get_experiment_groups("test_max_dynamic_config_size_again")
  end

  test "returns an empty list for an inactive experiment" do
    assert {:ok, []} = Statsig.get_experiment_groups("an_experiment1")
  end

  defmodule MockScrapi do
    defstruct [:pid, :socket, :specs_url]

    def start_link(response_body) do
      {:ok, socket} =
        :gen_tcp.listen(0, [
          :binary,
          active: false,
          packet: :raw,
          reuseaddr: true,
          ip: {127, 0, 0, 1}
        ])

      {:ok, {{127, 0, 0, 1}, port}} = :inet.sockname(socket)
      pid = spawn_link(fn -> accept_loop(socket, response_body) end)

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

    defp accept_loop(socket, response_body) do
      case :gen_tcp.accept(socket) do
        {:ok, client} ->
          handle_client(client, response_body)
          accept_loop(socket, response_body)

        {:error, _reason} ->
          :ok
      end
    end

    defp handle_client(client, response_body) do
      case :gen_tcp.recv(client, 0, 5_000) do
        {:ok, _request} ->
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
end
