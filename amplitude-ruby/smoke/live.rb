# frozen_string_literal: true

require 'amplitude'
require 'minitest/autorun'
require 'securerandom'
require 'timeout'

class LiveSmokeTest < Minitest::Test
  def test_sends_live_identify_and_track
    flush_interval_ms = 3_000
    client = live_client(flush_interval_ms)
    run_id = "ruby-#{SecureRandom.uuid}"
    user_id = "sdk-smoke-#{run_id}"
    puts "Ruby smoke run #{run_id}, user ID #{user_id}"
    properties = { smoke_run_id: run_id, sdk: 'ruby' }
    operations = Amplitude::Identify.new.set('sdk_smoke_run_id', run_id)
    completed = Queue.new
    callback = ->(_event, code, message) { completed << [code, message] }

    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    Timeout.timeout(2) do
      assert_nil client.identify(operations, user_id: user_id, event_properties: properties, callback: callback)
      (1..3).each do |number|
        assert_nil client.track(
          'SDK Smoke Test',
          user_id: user_id,
          event_properties: properties.merge(track_number: number),
          callback: callback
        )
      end
    end

    results = Timeout.timeout(15) { Array.new(4) { completed.pop } }
    assert_equal [200] * 4, results.map(&:first), results.filter_map(&:last).join(', ')
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
    assert_operator elapsed, :>=, flush_interval_ms / 1_000.0
    puts 'Amplitude accepted the Ruby smoke batch after the three-second timer (HTTP 200)'
  ensure
    client&.shutdown
  end

  def test_sends_live_event_on_shutdown
    client = live_client(10_000)
    run_id = "ruby-shutdown-#{SecureRandom.uuid}"
    user_id = "sdk-smoke-#{run_id}"
    puts "Ruby shutdown smoke run #{run_id}, user ID #{user_id}"
    completed = Queue.new

    code, message = Timeout.timeout(9) do
      assert_nil(client.track(
        'SDK Smoke Test Shutdown',
        user_id: user_id,
        event_properties: { smoke_run_id: run_id, sdk: 'ruby' }
      ) { |_, code, message| completed << [code, message] })
      client.shutdown
      completed.pop
    end

    assert_equal 200, code, message
    puts 'Amplitude accepted the Ruby shutdown event (HTTP 200)'
  end

  private

  def live_client(flush_interval_ms)
    Amplitude.new(
      ENV.fetch('AMPLITUDE_API_KEY'),
      flush_interval_millis: flush_interval_ms,
      server_zone: ENV.fetch('AMPLITUDE_SERVER_ZONE', 'us')
    )
  end
end
