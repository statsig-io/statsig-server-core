defmodule Statsig.Test.MockScrapi do
  @moduledoc """
  A minimal TCP server that responds to every request with the given body,
  used to serve download_config_specs payloads in tests.
  """

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
        _ = :gen_tcp.recv(client, 0, 5_000)
        :gen_tcp.send(client, http_response(response_body))
        :gen_tcp.close(client)
        accept_loop(socket, response_body)

      {:error, _reason} ->
        :ok
    end
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
