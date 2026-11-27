# frozen_string_literal: true

require 'minitest/autorun'
require 'socket'
require 'timeout'
require 'amplitude'

class ClientTest < Minitest::Test
  class TestPlugin < Amplitude::BasePlugin
    attr_reader :client

    def initialize(plugin_type, &execute)
      super(plugin_type)
      @execute = execute
    end

    def setup(client)
      @client = client
    end

    def execute(event)
      @execute.call(event)
    end
  end

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

  def test_tracks_using_an_event_hash
    _, payload = with_ingestion_server(200, ingestion_metadata: { source_name: 'default-source' }) do |client|
      assert_nil client.track(
        {
          'event_type' => 'Purchase Completed',
          'options' => { device_id: 'device-123', insert_id: 'event-123', product_id: 'shirt' },
          'groups' => { team: %w[north south] },
          'library' => 'custom-sdk/1.0'
        },
        insert_id: 'override-123'
      )
    end

    event = payload.fetch('events').fetch(0)
    assert_equal 'device-123', event.fetch('device_id')
    assert_equal 'override-123', event.fetch('insert_id')
    assert_equal 'shirt', event.fetch('productId')
    assert_equal({ 'source_name' => 'default-source' }, event.fetch('ingestion_metadata'))
    assert_equal({ 'team' => %w[north south] }, event.fetch('groups'))
    assert_equal 'custom-sdk/1.0', event.fetch('library')
  end

  def test_track_reports_ingestion_errors_to_the_callback
    _, payload = with_ingestion_server(400) do |client|
      completions = []
      error = assert_raises(Amplitude::Error) do
        client.track('App Opened', device_id: 'device-123', callback: ->(*args) { completions << args })
      end
      assert_equal 'Amplitude returned HTTP 400', error.message
      assert_equal [
        [{ 'event_type' => 'App Opened', 'options' => { 'device_id' => 'device-123' } }, 400, error.message]
      ], completions
    end

    event = payload.fetch('events').fetch(0)
    assert_equal 'device-123', event.fetch('device_id')
    refute event.key?('user_id')
    assert_operator event.fetch('time'), :>, 0
    assert_match(/\A[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}\z/, event.fetch('insert_id'))
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

  def test_identifies_and_reports_delivery
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
      completions = []
      assert_nil(client.identify(
        operations,
        { app_version: '1.2' },
        user_id: 'user-123',
        event_properties: { source: 'account' },
        time: 1_234_567_890_000,
        insert_id: 'event-123',
        session_id: 1_234_567_890_000
      ) { |*args| completions << args })
      assert_equal 1, completions.size
      event, status, message = completions.first
      assert_equal '$identify', event.fetch('event_type')
      assert_equal 'user-123', event.fetch('options').fetch('user_id')
      assert_equal({ 'plan' => 'pro' }, event.fetch('user_properties').fetch('$set'))
      assert_equal 200, status
      assert_nil message
    end

    assert_equal "POST /2/httpapi HTTP/1.1\r\n", request
    assert_equal 'test-api-key', payload.fetch('api_key')
    event = payload.fetch('events').fetch(0)
    assert_equal 1, payload.fetch('events').length
    assert_equal '$identify', event.fetch('event_type')
    assert_match(/\Aamplitude-ruby-core\//, event.fetch('library'))
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

  def test_client_runs_plugins_in_stage_order_and_can_remove_them
    _, payload = with_ingestion_server(200) do |client|
      before = TestPlugin.new(:before) do |event|
        event['options']['user_id'] = 'user-123'
        event['event_properties'] = { 'stages' => ['before'] }
        event
      end
      enrichment = TestPlugin.new(:enrichment) do |event|
        event['event_properties']['stages'] << 'enrichment'
        event
      end
      removed = TestPlugin.new(:before) { flunk 'Removed plugin was executed' }
      client.add(enrichment).add(before).add(removed).remove(removed)

      assert_same client, before.client
      assert_equal 3_000, before.client.configuration.timeout_ms
      assert_nil(client.track('App Opened') do |event, code, message|
        assert_equal({}, event.fetch('options'))
        assert_equal 200, code
        assert_nil message
      end)
    end
    event = payload.fetch('events').first
    assert_equal 'user-123', event.fetch('user_id')
    assert_equal %w[before enrichment], event.dig('event_properties', 'stages')
  end

  def test_timeline_filters_events_and_returns_first_destination_result
    timeline = Amplitude::Timeline.new
    event = { 'event_type' => 'App Opened', 'options' => {} }
    result = timeline.apply(event)
    assert_equal 100, result.code
    assert_equal 'Event not tracked, no destination plugins on the instance', result.message

    calls = []
    first = TestPlugin.new(:destination) do |current|
      calls << :first
      current['event_type'] = 'Only first sees this'
      Amplitude::Result.new(event: current, code: 200, message: '')
    end
    second = TestPlugin.new(:destination) do |current|
      calls << :second
      assert_equal 'App Opened', current.fetch('event_type')
      Amplitude::Result.new(event: current, code: 400, message: 'Rejected')
    end
    timeline.add(first)
    timeline.add(second)
    assert_equal 200, timeline.apply(event).code
    assert_equal %i[first second], calls

    timeline.add(TestPlugin.new(:enrichment) { nil })
    assert_equal 0, timeline.apply(event).code
    assert_equal %i[first second], calls
  end

  def test_opt_out_completes_without_http_status
    client = Amplitude.new('test-api-key', opt_out: true)
    completion = nil
    assert_nil(client.track('App Opened') { |*args| completion = args })
    assert_equal [{ 'event_type' => 'App Opened', 'options' => {} }, 0, nil], completion
  end

  def test_queue_settings_batch_sequential_calls
    _, payload = with_ingestion_server(200, flush_interval_millis: 3_000, flush_queue_size: 2) do |client|
      Timeout.timeout(1) do
        assert_nil client.track('First', device_id: 'device-123')
        assert_nil client.identify(Amplitude::Identify.new.set('plan', 'pro'), device_id: 'device-123')
      end
    end
    assert_equal ['First', '$identify'], payload.fetch('events').map { |event| event.fetch('event_type') }
  end

  def test_track_returns_before_a_five_second_flush_interval
    with_ingestion_server(200, flush_interval_millis: 5_000) do |client|
      completed = Queue.new
      caller_thread = Thread.current
      Timeout.timeout(1) do
        assert_nil(client.track('App Opened', device_id: 'device-123') do |event, code, message|
          client.flush
          completed << [event.fetch('event_type'), code, message, Thread.current]
        end)
      end

      assert_empty completed
      assert_equal 200, client.flush.first.first.code
      name, code, message, callback_thread = Timeout.timeout(5) { completed.pop }
      assert_equal ['App Opened', 200, nil], [name, code, message]
      refute_same caller_thread, callback_thread
    end
  end

  def test_flush_interval_sends_a_partial_batch_and_reports_errors_asynchronously
    _, payload = with_ingestion_server(400, flush_interval_millis: 20, flush_queue_size: 2) do |client|
      completed = Queue.new
      started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
      assert_nil(client.track('App Opened', device_id: 'device-123') { |_, code, message| completed << [code, message] })
      assert_equal [400, 'Amplitude returned HTTP 400'], Timeout.timeout(5) { completed.pop }
      assert_operator Process.clock_gettime(Process::CLOCK_MONOTONIC) - started, :>=, 0.02
    end
    assert_equal 1, payload.fetch('events').length
  end

  def test_retries_with_the_same_payload_and_reports_the_final_status
    completions = []
    requests = with_ingestion_responses([503, 200], flush_max_retries: 1) do |client|
      assert_nil(client.track('App Opened', device_id: 'device-123') { |*args| completions << args })
    end
    assert_equal requests[0], requests[1]
    assert_equal [
      [{ 'event_type' => 'App Opened', 'options' => { 'device_id' => 'device-123' } }, 200, nil]
    ], completions
  end

  def test_flush_and_shutdown_drain_pending_events_before_the_interval
    requests = with_ingestion_responses([200, 200], flush_interval_millis: 60_000) do |client|
      destination = Amplitude::AmplitudeDestinationPlugin.new
      client.add(destination)
      event = { 'event_type' => 'App Opened', 'options' => { 'device_id' => 'device-123' } }

      pending = destination.execute(event)
      results = Timeout.timeout(5) { client.flush }
      assert_equal [[], [pending.value]], results
      assert_equal 200, pending.value.code

      pending = destination.execute(event)
      Timeout.timeout(5) { client.shutdown }
      assert_equal 200, pending.value.code
      assert_raises(Amplitude::Error) { client.track('App Opened', device_id: 'device-123') }
    end
    assert_equal [1, 1], requests.map { |_, payload| payload.fetch('events').length }
  end

  private

  def with_ingestion_server(status, **configuration)
    with_ingestion_responses([status], **configuration) { |client| yield client }.first
  end

  def with_ingestion_responses(statuses, **configuration)
    server = TCPServer.new('127.0.0.1', 0)
    worker = Thread.new do
      statuses.map do |status|
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
    end
    client = Amplitude.new(
      'test-api-key',
      server_url: "http://127.0.0.1:#{server.addr[1]}/2/httpapi",
      timeout_ms: 3_000,
      **configuration
    )
    yield client
    Timeout.timeout(5) { worker.value }
  ensure
    worker&.kill&.join
    server&.close
  end
end
