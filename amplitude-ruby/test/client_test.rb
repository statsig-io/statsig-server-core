# frozen_string_literal: true

require 'minitest/autorun'
require 'socket'
require 'timeout'
require 'amplitude'

class ClientTest < Minitest::Test
  def test_tracks_using_event_name_and_keywords
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
    assert_equal 1, payload.fetch('events').length
    event = payload.fetch('events').fetch(0)
    assert_equal 'Purchase Completed', event.fetch('event_type')
    assert_match(%r{\Aamplitude-ruby-core/\d+\.\d+\.\d+(?:[-+][\w.+-]+)?\z}, event.fetch('library'))
    assert_equal 'user-123', event.fetch('user_id')
    assert_equal({ 'total' => 29.99, 'items' => ['shirt'] }, event.fetch('event_properties'))
    assert_equal({ 'plan' => 'free' }, event.fetch('user_properties'))
    assert_equal 1_234_567_890_000, event.fetch('time')
    assert_equal 1_234_567_890_000, event.fetch('session_id')
    assert_equal 'event-123', event.fetch('insert_id')
  end

  def test_tracks_using_a_wit_event_hash
    _, payload = with_ingestion_server(200) do |client|
      assert_nil client.track(
        {
          'event_type' => 'Purchase Completed',
          'options' => { device_id: 'device-123', insert_id: 'event-123' },
          'groups' => { team: %w[north south] },
          'library' => 'custom-sdk/1.0'
        },
        insert_id: 'override-123'
      )
    end

    event = payload.fetch('events').fetch(0)
    assert_equal 'device-123', event.fetch('device_id')
    assert_equal 'override-123', event.fetch('insert_id')
    assert_equal({ 'team' => %w[north south] }, event.fetch('groups'))
    assert_equal 'custom-sdk/1.0', event.fetch('library')
  end

  def test_track_reports_ingestion_errors_for_device_only_events
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

  def test_identifies_through_the_native_client
    request, payload = with_ingestion_server(200) do |client|
      operations = Amplitude::Identify.new
        .set('plan', 'pro')
        .set_once('signup', 'email')
        .append('tags', %w[ruby rust])
        .prepend('history', 'latest')
        .pre_insert('segments', 'early')
        .post_insert('cohorts', 'beta')
        .remove('flags', 'legacy')
        .add('logins', 2.5)
        .unset('old_plan')
      assert_nil client.identify(
        operations,
        { app_version: '1.2' },
        user_id: 'user-123',
        event_properties: { source: 'account' },
        time: 1_234_567_890_000,
        insert_id: 'event-123',
        session_id: 1_234_567_890_000,
        callback: 3
      )
    end

    assert_equal "POST /2/httpapi HTTP/1.1\r\n", request
    assert_equal 'test-api-key', payload.fetch('api_key')
    event = payload.fetch('events').fetch(0)
    assert_equal 1, payload.fetch('events').length
    assert_equal '$identify', event.fetch('event_type')
    assert_equal 'user-123', event.fetch('user_id')
    assert_equal({ 'source' => 'account' }, event.fetch('event_properties'))
    assert_equal(
      {
        '$set' => { 'plan' => 'pro' },
        '$setOnce' => { 'signup' => 'email' },
        '$append' => { 'tags' => %w[ruby rust] },
        '$prepend' => { 'history' => 'latest' },
        '$preInsert' => { 'segments' => 'early' },
        '$postInsert' => { 'cohorts' => 'beta' },
        '$remove' => { 'flags' => 'legacy' },
        '$add' => { 'logins' => 2.5 },
        '$unset' => { 'old_plan' => '-' }
      },
      event.fetch('user_properties')
    )
    assert_equal 1_234_567_890_000, event.fetch('time')
    assert_equal 1_234_567_890_000, event.fetch('session_id')
    assert_equal 'event-123', event.fetch('insert_id')
    assert_equal '1.2', event.fetch('app_version')
    refute event.key?('callback')
  end

  def test_reports_ingestion_errors_and_supports_device_only_events
    _, payload = with_ingestion_server(400) do |client|
      operations = Amplitude::Identify.new.clear_all
      error = assert_raises(Amplitude::Error) { client.identify(operations, device_id: 'device-123') }
      assert_equal 'Amplitude returned HTTP 400', error.message
    end

    event = payload.fetch('events').fetch(0)
    assert_equal 'device-123', event.fetch('device_id')
    assert_equal({ '$clearAll' => '-' }, event.fetch('user_properties'))
    refute event.key?('user_id')
    assert_operator event.fetch('time'), :>, 0
    assert_match(/\A[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}\z/, event.fetch('insert_id'))
  end

  def test_raises_for_invalid_configuration_and_events
    assert_raises(Amplitude::Error) { Amplitude.new('') }
    assert_raises(Amplitude::Error) { Amplitude.new('test-api-key', timeout_ms: -1) }
    client = Amplitude.new('test-api-key')
    assert_raises(Amplitude::Error) { client.track('App Opened') }
    operations = Amplitude::Identify.new.set('plan', 'pro')
    assert_raises(Amplitude::Error) { client.identify(operations) }
    assert_raises(Amplitude::Error) { client.identify(operations.clear_all, user_id: 'user-123') }
  end

  def test_other_client_methods_raise_not_implemented
    client = Amplitude.new('test-api-key')
    stubs = {
      configuration: [],
      'configuration=': [{}],
      api_key: [],
      'api_key=': ['next-api-key'],
      group_identify: ['company', 'example', Amplitude::Identify.new],
      revenue: [{ price: 2.5 }],
      set_group: ['company', ['example']],
      flush: [],
      add: [1, :before],
      remove: [1],
      shutdown: []
    }
    stubs.each do |name, args|
      error = assert_raises(NotImplementedError) { client.public_send(name, *args) }
      assert_equal 'Not implemented', error.message
    end
    assert_raises(NotImplementedError) { Amplitude::FlushTask.new.wait }
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
