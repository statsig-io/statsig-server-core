defmodule Statsig.GrpcSpecAdapterTest do
  @moduledoc """
  Regression coverage for the `with_grpc` cargo feature in the Elixir NIF.

  Without that feature, the gRPC adapter is skipped and nothing dials the
  configured URL. With it, the native client opens a TCP connection during
  initialize. A local listener is enough: the assertion is the connection,
  not a successful spec sync.
  """
  use ExUnit.Case, async: false

  @tag capture_log: false
  test "grpc websocket adapter dials the configured endpoint" do
    parent = self()

    {:ok, socket} =
      :gen_tcp.listen(0, [
        :binary,
        active: false,
        packet: :raw,
        reuseaddr: true,
        ip: {127, 0, 0, 1}
      ])

    {:ok, {{127, 0, 0, 1}, port}} = :inet.sockname(socket)

    # Accept before initialize. The client connects and may drop the socket
    # while initialize is still waiting on its timeout.
    acceptor =
      spawn_link(fn ->
        case :gen_tcp.accept(socket, 5_000) do
          {:ok, client} ->
            send(parent, {:grpc_connected, client})
            :gen_tcp.close(client)

          {:error, reason} ->
            send(parent, {:grpc_accept_failed, reason})
        end
      end)

    on_exit(fn ->
      Statsig.shutdown()

      if pid = Process.whereis(Statsig) do
        Process.exit(pid, :normal)
      end

      if Process.alive?(acceptor), do: Process.exit(acceptor, :shutdown)
      :gen_tcp.close(socket)
    end)

    options = %Statsig.Options{
      disable_all_logging: true,
      fallback_to_statsig_api: false,
      spec_adapter_configs: [
        %Statsig.SpecAdapterConfig{
          adapter_type: "network_grpc_websocket",
          specs_url: "http://127.0.0.1:#{port}",
          init_timeout_ms: 1_000,
          authentication_mode: "none"
        }
      ]
    }

    assert {:ok, _pid} = Statsig.start_link("secret-key", options)

    # Initialize times out: nothing on this port speaks gRPC. That is expected.
    Statsig.initialize()

    assert_receive {:grpc_connected, client}, 2_000
    assert is_port(client)
  end
end
