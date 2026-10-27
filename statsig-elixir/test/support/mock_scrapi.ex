defmodule Statsig.Test.MockScrapi do
  @moduledoc """
  A minimal TCP server that responds to every request with the given body,
  used to serve download_config_specs payloads in tests.

  Pass `notify: pid` to forward each request to `pid` as
  `{:mock_scrapi_request, path, headers, body}`.
  """

  defstruct [:pid, :socket, :specs_url, :log_event_url]

  def start_link(response_body, opts \\ []) do
    {:ok, socket} =
      :gen_tcp.listen(0, [
        :binary,
        active: false,
        packet: :raw,
        reuseaddr: true,
        ip: {127, 0, 0, 1}
      ])

    {:ok, {{127, 0, 0, 1}, port}} = :inet.sockname(socket)
    pid = spawn_link(fn -> accept_loop(socket, response_body, opts[:notify]) end)

    {:ok,
     %__MODULE__{
       pid: pid,
       socket: socket,
       specs_url: "http://127.0.0.1:#{port}/v2/download_config_specs",
       log_event_url: "http://127.0.0.1:#{port}/v1/log_event"
     }}
  end

  def stop(%__MODULE__{pid: pid, socket: socket}) do
    :gen_tcp.close(socket)

    if Process.alive?(pid) do
      Process.unlink(pid)
      Process.exit(pid, :shutdown)
    end
  end

  defp accept_loop(socket, response_body, notify) do
    case :gen_tcp.accept(socket) do
      {:ok, client} ->
        case :gen_tcp.recv(client, 0, 5_000) do
          {:ok, data} when is_pid(notify) -> notify_request(client, data, notify)
          _ -> :ok
        end

        :gen_tcp.send(client, http_response(response_body))
        :gen_tcp.close(client)
        accept_loop(socket, response_body, notify)

      {:error, _reason} ->
        :ok
    end
  end

  defp notify_request(client, data, notify) do
    {head, body} = read_head(client, data)
    [request_line | header_lines] = String.split(head, "\r\n")
    [_method, path | _] = String.split(request_line, " ")

    headers =
      Map.new(header_lines, fn line ->
        [key, value] = String.split(line, ":", parts: 2)
        {String.downcase(key), String.trim(value)}
      end)

    content_length = String.to_integer(Map.get(headers, "content-length", "0"))
    body = read_body(client, body, content_length)

    send(notify, {:mock_scrapi_request, path, headers, body})
  end

  defp read_head(client, data) do
    case :binary.split(data, "\r\n\r\n") do
      [head, body] ->
        {head, body}

      [_] ->
        {:ok, more} = :gen_tcp.recv(client, 0, 5_000)
        read_head(client, data <> more)
    end
  end

  defp read_body(_client, body, content_length) when byte_size(body) >= content_length,
    do: body

  defp read_body(client, body, content_length) do
    {:ok, more} = :gen_tcp.recv(client, 0, 5_000)
    read_body(client, body <> more, content_length)
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
