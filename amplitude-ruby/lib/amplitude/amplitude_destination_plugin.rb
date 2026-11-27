# frozen_string_literal: true

require 'net/http'
require 'securerandom'
require 'timeout'

class Amplitude
  class AmplitudeDestinationPlugin < DestinationPlugin
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
    Entry = Struct.new(:event, :payload)
    Batch = Struct.new(:entries, :deadline, :worker) do
      def initialize(entries, deadline)
        super
        @callback_lock = Mutex.new
        @callbacks = []
      end

      def on_complete(index, &callback)
        @callback_lock.synchronize do
          if @callbacks
            @callbacks << [index, callback]
            # One notifier per batch. Callbacks can flush the client because
            # the upload worker never waits for the notifier.
            Thread.new { notify_callbacks } if @callbacks.length == 1
            return
          end
        end
        Thread.new { callback.call(worker.value.fetch(index)) }
      end

      private

      def notify_callbacks
        results = worker.value
        callbacks = @callback_lock.synchronize do
          pending = @callbacks
          @callbacks = nil
          pending
        end
        callbacks.each { |index, callback| callback.call(results.fetch(index)) }
      end
    end

    DeliveryTask = Struct.new(:batch, :index) do
      def value
        batch.worker.value.fetch(index)
      end

      def on_complete(&callback)
        batch.on_complete(index, &callback)
      end
    end

    private_constant :LIBRARY, :RETRYABLE_STATUSES, :NETWORK_FAILURES, :RETRYABLE_NETWORK_FAILURES,
                     :Entry, :Batch, :DeliveryTask

    def initialize
      super
      @mutex = Mutex.new
      @ready = ConditionVariable.new
      @active = []
      @pending = nil
      @closed = false
    end

    def setup(client)
      @configuration = client.configuration
      nil
    end

    def execute(event)
      raise Error, 'Destination must be set up before use' unless @configuration

      event = Payload.event(event)
      Payload.validate_event(event)
      enqueue(Entry.new(event, ingestion_event(event)))
    rescue Error => failure
      Result.new(event: event, code: failure.status_code, message: failure.message)
    end

    def flush
      drain
    end

    def shutdown
      drain(close: true)
      nil
    end

    private

    def enqueue(entry)
      @mutex.synchronize do
        raise Error, 'Destination has been shut down' if @closed

        now = monotonic_time
        if @pending.nil? || @pending.deadline <= now
          batch = Batch.new([entry], now + @configuration.flush_interval_millis / 1000.0)
          batch.worker = Thread.new(batch) { |pending| send_batch(pending) }
          @active << batch
          @pending = batch
        else
          @pending.entries << entry
        end

        batch = @pending
        index = batch.entries.length - 1
        if batch.entries.length == @configuration.flush_queue_size
          @pending = nil
          @ready.broadcast
        end
        DeliveryTask.new(batch, index)
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
        batch.entries
      end
    end

    def send_batch(batch)
      entries = close_batch(batch)
      code, message = upload(entries.map(&:payload))
      entries.map { |entry| Result.new(event: entry.event, code: code, message: message) }
    ensure
      @mutex.synchronize { @active.delete(batch) }
    end

    def drain(close: false)
      workers = @mutex.synchronize do
        @closed = true if close
        @pending = nil
        @ready.broadcast
        @active.map(&:worker)
      end
      workers.flat_map(&:value)
    end

    def monotonic_time
      Process.clock_gettime(Process::CLOCK_MONOTONIC)
    end

    def ingestion_event(event)
      options = event.fetch('options').dup
      options['time'] ||= (Time.now.to_r * 1000).to_i
      options['insert_id'] ||= SecureRandom.uuid
      options['plan'] ||= @configuration.plan
      options['ingestion_metadata'] ||= @configuration.ingestion_metadata
      Payload::WIRE_NAMES.each { |name, wire| options[wire] = options.delete(name) if options.key?(name) }
      { 'library' => LIBRARY }.merge(event.except('options'), options.compact)
    end

    def upload(events)
      body = JSON.generate(api_key: @configuration.api_key, events: events)
      @configuration.logger.debug("Uploading #{events.length} event(s)")
      retries = 0
      loop do
        status, message, retryable = attempt(body)
        if message.nil?
          @configuration.logger.info("Uploaded #{events.length} event(s)")
          return [status, '']
        end
        if retries == @configuration.flush_max_retries || !retryable
          @configuration.logger.error(message)
          return [status, message]
        end
        @configuration.logger.warn("Retrying upload after #{message}")
        sleep((100 << [retries, 4].min) / 1000.0)
        retries += 1
      end
    end

    def attempt(body)
      uri = @configuration.upload_uri
      timeout = @configuration.timeout_ms / 1000.0
      request = Net::HTTP::Post.new(uri.request_uri, 'Content-Type' => 'application/json')
      request.body = body
      http = Net::HTTP.new(uri.hostname, uri.port)
      http.use_ssl = uri.scheme == 'https'
      http.open_timeout = http.read_timeout = http.write_timeout = timeout
      http.max_retries = 0
      status = Timeout.timeout(timeout) { http.start { |session| session.request(request).code.to_i } }
      message = "Amplitude returned HTTP #{status}" unless status == 200
      [status, message, RETRYABLE_STATUSES.include?(status)]
    rescue *NETWORK_FAILURES => error
      retryable = RETRYABLE_NETWORK_FAILURES.any? { |type| error.is_a?(type) }
      [0, "Amplitude HTTP client error: #{error.class}", retryable]
    end
  end
end
