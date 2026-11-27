# frozen_string_literal: true

class Amplitude
  class Timeline
    def initialize(configuration = nil)
      @configuration = configuration
      @plugins = { before: [], enrichment: [], destination: [] }
      @mutex = Mutex.new
      @closed = false
    end

    def setup(client)
      @configuration = client.configuration
      nil
    end

    def logger
      @configuration&.logger
    end

    def add(plugin)
      @mutex.synchronize do
        raise Error, 'Timeline has been shut down' if @closed

        @plugins.fetch(plugin.plugin_type.to_sym) << plugin
      end
      nil
    end

    def remove(plugin)
      @mutex.synchronize { @plugins.each_value { |stage| stage.delete(plugin) } }
      nil
    end

    def apply(event, wait: true)
      before, enrichment, destinations = @mutex.synchronize do
        raise Error, 'Timeline has been shut down' if @closed

        @plugins.values.map(&:dup)
      end
      return Result.new(event: event, code: 0, message: '') if @configuration&.opt_out

      (before + enrichment).each do |plugin|
        processed = plugin.execute(event)
        return Result.new(event: event, code: 0, message: '') if processed.nil?

        event = processed
      end

      if destinations.empty?
        return Result.new(event: event, code: 100, message: 'Event not tracked, no destination plugins on the instance')
      end

      results = destinations.map { |plugin| plugin.execute(Payload.snapshot(event)) }
      first = results.first
      wait && first.respond_to?(:value) ? first.value : first
    end

    def flush
      @mutex.synchronize { @plugins.fetch(:destination).dup }.map(&:flush)
    end

    def shutdown
      destinations = @mutex.synchronize do
        return if @closed

        @closed = true
        @plugins.fetch(:destination).dup
      end
      destinations.each(&:shutdown)
      nil
    end
  end
end
