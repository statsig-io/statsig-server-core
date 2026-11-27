# frozen_string_literal: true

require 'net/http'
require 'securerandom'
require 'timeout'

class Amplitude
  class Delivery
    LIBRARY = 'amplitude-ruby-core/0.24.4'
    RETRYABLE_STATUSES = [408, 429, 500, 502, 503, 504].freeze
    NETWORK_FAILURES = [
      Timeout::Error, SocketError, SystemCallError, IOError, OpenSSL::SSL::SSLError,
      Net::HTTPBadResponse, Net::ProtocolError
    ].freeze
    RETRYABLE_NETWORK_FAILURES = [
      Timeout::Error, SocketError, Errno::ECONNREFUSED, Errno::ECONNRESET, Errno::ETIMEDOUT,
      Errno::EHOSTUNREACH, Errno::ENETUNREACH, EOFError
    ].freeze
    LOG_LEVELS = %w[None Error Warn Verbose Debug].freeze
    Batch = Struct.new(:events, :deadline, :worker)

    attr_reader :opt_out

    def initialize(api_key, config)
      unless api_key.is_a?(String) && !api_key.strip.empty?
        raise Error, 'api_key must be a nonblank string'
      end

      @api_key = api_key.dup
      config = Payload.snapshot(config).compact
      @opt_out = boolean(config.fetch('opt_out', false), 'opt_out')
      use_batch = boolean(config.fetch('use_batch', false), 'use_batch')
      zone = config.fetch('server_zone', 'us')
      raise Error, 'server_zone must be us or eu' unless %w[us eu].include?(zone)

      default_host = zone == 'eu' ? 'api.eu.amplitude.com' : 'api2.amplitude.com'
      @uri = endpoint(config.fetch('server_url', "https://#{default_host}#{use_batch ? '/batch' : '/2/httpapi'}"))
      timeout = config.fetch('timeout_ms', 10_000)
      unless timeout.is_a?(Integer) && (1...2**64).cover?(timeout)
        raise Error, 'timeout_ms must be a positive integer'
      end

      @timeout = timeout / 1000.0
      interval = config.fetch('flush_interval_millis', 0)
      @interval = interval / 1000.0 if interval.is_a?(Numeric)
      unless @interval&.finite? && @interval >= 0
        raise Error, 'flush_interval_millis must be a finite nonnegative duration'
      end

      @capacity = count(config.fetch('flush_queue_size', 200), 1, 'flush_queue_size')
      @retries = count(config.fetch('flush_max_retries', 0), 0, 'flush_max_retries')
      @log_level = config.fetch('log_level', 0)
      unless @log_level.is_a?(Integer) && (0...LOG_LEVELS.length).cover?(@log_level)
        raise Error, 'log_level must be an integer between 0 and 4'
      end

      @name = config.fetch('instance_name', 'Amplitude')
      raise Error, 'instance_name must be a string' unless @name.is_a?(String)

      @plan = Payload.metadata(config['plan'], 'plan')
      @ingestion_metadata = Payload.metadata(config['ingestion_metadata'], 'ingestion_metadata')
      @mutex = Mutex.new
      @ready = ConditionVariable.new
      @pending = nil
    end

    def submit(event)
      event = ingestion_event(event)
      return upload([event]) if @interval.zero? || @capacity == 1

      enqueue(event).worker.value
    end

    private

    def endpoint(url)
      uri = URI.parse(url) if url.is_a?(String)
      return uri if uri.is_a?(URI::HTTP) && uri.host && !uri.host.empty? && (1..65_535).cover?(uri.port)

      raise Error, 'server_url must be a valid HTTP(S) URL'
    rescue URI::Error
      raise Error, 'server_url must be a valid HTTP(S) URL'
    end

    def boolean(value, name)
      return value if value == true || value == false

      raise Error, "#{name} must be a boolean"
    end

    def count(value, minimum, name)
      if value.is_a?(Numeric) && (minimum...2**32).cover?(value) && value == value.to_i
        return value.to_i
      end

      raise Error, "#{name} must be a #{minimum.zero? ? 'nonnegative' : 'positive'} whole number"
    end

    def ingestion_event(event)
      options = event.fetch('options').dup
      options['time'] ||= (Time.now.to_r * 1000).to_i
      options['insert_id'] ||= SecureRandom.uuid
      options['plan'] ||= @plan
      options['ingestion_metadata'] ||= @ingestion_metadata
      Payload::WIRE_NAMES.each { |name, wire| options[wire] = options.delete(name) if options.key?(name) }
      { 'library' => LIBRARY }.merge(event.except('options'), options.compact)
    end

    def enqueue(event)
      @mutex.synchronize do
        now = monotonic_time
        if @pending.nil? || @pending.deadline <= now
          batch = Batch.new([event], now + @interval)
          # One worker owns the upload, so an interrupted caller cannot strand the batch.
          batch.worker = Thread.new(batch) { |pending| upload(close_batch(pending)) }
          @pending = batch
        else
          @pending.events << event
        end
        batch = @pending
        if batch.events.length == @capacity
          @pending = nil
          @ready.broadcast
        end
        batch
      end
    end

    def close_batch(batch)
      @mutex.synchronize do
        while @pending.equal?(batch)
          remaining = batch.deadline - monotonic_time
          break unless remaining.positive?

          @ready.wait(@mutex, [remaining, 3600].min)
        end
        @pending = nil if @pending.equal?(batch)
        batch.events
      end
    end

    def monotonic_time
      Process.clock_gettime(Process::CLOCK_MONOTONIC)
    end

    def upload(events)
      body = JSON.generate(api_key: @api_key, events: events)
      log(4, "Uploading #{events.length} event(s)")
      retries = 0
      loop do
        status, message, retryable = attempt(body)
        if message.nil?
          log(3, "Uploaded #{events.length} event(s)")
          return [status, nil]
        end
        if retries == @retries || !retryable
          log(1, message)
          return [status, message]
        end
        log(2, "Retrying upload after #{message}")
        sleep((100 << [retries, 4].min) / 1000.0)
        retries += 1
      end
    end

    def attempt(body)
      request = Net::HTTP::Post.new(@uri.request_uri, 'Content-Type' => 'application/json')
      request.body = body
      http = Net::HTTP.new(@uri.hostname, @uri.port)
      http.use_ssl = @uri.scheme == 'https'
      http.open_timeout = http.read_timeout = http.write_timeout = @timeout
      http.max_retries = 0
      status = Timeout.timeout(@timeout) { http.start { |session| session.request(request).code.to_i } }
      message = "Amplitude returned HTTP #{status}" unless status == 200
      [status, message, RETRYABLE_STATUSES.include?(status)]
    rescue *NETWORK_FAILURES => error
      retryable = RETRYABLE_NETWORK_FAILURES.any? { |type| error.is_a?(type) }
      [0, "Amplitude HTTP client error: #{error.class}", retryable]
    end

    def log(level, message)
      warn("[#{@name}] #{LOG_LEVELS[level]}: #{message}") if @log_level >= level
    end
  end

  private_constant :Delivery
end
