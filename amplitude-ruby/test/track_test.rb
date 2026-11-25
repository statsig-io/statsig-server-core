# frozen_string_literal: true

require 'minitest/autorun'
require 'socket'
require 'timeout'
require 'amplitude'

class TrackTest < Minitest::Test
  def test_tracks_through_the_native_client
    request, payload = with_ingestion_server(200) do |client|
      assert_nil client.track(
        'Purchase Completed',
        user_id: 'user-123',
        event_properties: { total: 29.99, items: ['shirt'] },
        user_properties: { plan: 'free' },
        time: 1_234_567_890_000,
        insert_id: 'event-123',
        session_id: 1_234_567_890_000
      )
    end

    assert_equal "POST /2/httpapi HTTP/1.1\r\n", request
    assert_equal 'test-api-key', payload.fetch('api_key')
    event = payload.fetch('events').fetch(0)
    assert_equal 1, payload.fetch('events').length
    assert_equal 'Purchase Completed', event.fetch('event_type')
    assert_equal 'user-123', event.fetch('user_id')
    assert_equal({ 'total' => 29.99, 'items' => ['shirt'] }, event.fetch('event_properties'))
    assert_equal({ 'plan' => 'free' }, event.fetch('user_properties'))
    assert_equal 1_234_567_890_000, event.fetch('time')
    assert_equal 1_234_567_890_000, event.fetch('session_id')
    assert_equal 'event-123', event.fetch('insert_id')
  end

  def test_reports_ingestion_errors_and_supports_device_only_events
    _, payload = with_ingestion_server(400) do |client|
      error = assert_raises(Amplitude::Error) { client.track('App Opened', device_id: 'device-123') }
      assert_equal 'Amplitude returned HTTP 400', error.message
    end

    event = payload.fetch('events').fetch(0)
    assert_equal 'device-123', event.fetch('device_id')
    refute event.key?('user_id')
    assert_operator event.fetch('time'), :>, 0
    assert_match(/\A[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}\z/, event.fetch('insert_id'))
  end

  def test_raises_for_invalid_configuration_and_events
    assert_raises(Amplitude::Error) { Amplitude.new('') }
    assert_raises(Amplitude::Error) { Amplitude.new('test-api-key', timeout_ms: -1) }
    client = Amplitude.new('test-api-key')
    assert_raises(Amplitude::Error) { client.track('App Opened') }
  end

  private

  def with_ingestion_server(status)
    server = TCPServer.new('127.0.0.1', 0)
    worker = Thread.new do
      socket = server.accept
      request = socket.gets
      headers = {}
      while (line = socket.gets) && line != "\r\n"
        name, value = line.split(':', 2)
        headers[name.downcase] = value.strip
      end
      payload = JSON.parse(socket.read(Integer(headers.fetch('content-length'))))
      socket.write("HTTP/1.1 #{status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
      [request, payload]
    ensure
      socket&.close
    end
    client = Amplitude.new('test-api-key', server_url: "http://127.0.0.1:#{server.addr[1]}/2/httpapi", timeout_ms: 3_000)
    yield client
    Timeout.timeout(5) { worker.value }
  ensure
    worker&.kill&.join
    server&.close
  end
end
