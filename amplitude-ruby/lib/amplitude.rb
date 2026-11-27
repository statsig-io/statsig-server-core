# frozen_string_literal: true

require_relative 'amplitude/payload'
require_relative 'amplitude/identify'
require_relative 'amplitude/configuration'
require_relative 'amplitude/plugin'
require_relative 'amplitude/timeline'
require_relative 'amplitude/amplitude_destination_plugin'

class Amplitude
  class Error < StandardError
    attr_reader :status_code

    def initialize(message = nil, status_code: 0)
      super(message)
      @status_code = status_code
    end
  end

  attr_reader :configuration

  def initialize(api_key, configuration = nil, **configuration_overrides)
    @configuration = Configuration.new(api_key, merge_options(configuration, configuration_overrides))
    @timeline = Timeline.new
    @timeline.setup(self)
    add(AmplitudeDestinationPlugin.new)
  end

  def add(plugin)
    plugin.setup(self)
    @timeline.add(plugin)
    self
  end

  def remove(plugin)
    @timeline.remove(plugin)
    self
  end

  def flush
    @timeline.flush
  end

  def shutdown
    @timeline.shutdown
  end

  def track(event = {}, callback: nil, **fields, &block)
    payload = event.is_a?(Hash) ? event.transform_keys(&:to_s) : { 'event_type' => event }
    payload.merge!(fields.transform_keys(&:to_s))
    options = payload.delete('options')
    event = payload.slice(*Payload::EVENT_FIELDS)
    complete(event, callback || block) do
      event['options'] = merge_options(options, payload.except(*Payload::EVENT_FIELDS))
      event.replace(Payload.event(event))
    end
  end

  def identify(operations, options = nil, event_properties: nil, callback: nil, **option_overrides, &block)
    event = { 'event_type' => '$identify', 'event_properties' => event_properties }
    callback ||= block
    begin
      event['options'] = merge_options(options, option_overrides)
      begin
        event['user_properties'] = Identify.user_properties(operations)
      rescue Error
        raise unless @configuration.opt_out
      end
    rescue Error => failure
      callback&.call(event.compact, failure.status_code, failure.message)
      raise
    end
    track(event, callback: callback)
  end

  private

  def merge_options(options, overrides)
    options = Payload.object(options, 'options') unless options.nil?
    (options || {}).transform_keys(&:to_s).merge(overrides.transform_keys(&:to_s))
  end

  def complete(event, callback)
    error = nil
    status = begin
      yield
      result = @timeline.apply(Payload.snapshot(event), wait: @configuration.flush_interval_millis.zero?)
      if result.respond_to?(:value)
        notify_later(result, event, callback) if callback
        return
      end
      error = delivery_error(result)
      result.code
    rescue Error => failure
      error = failure
      failure.status_code
    end
    callback&.call(event, status, error&.message)
    raise error if error

    nil
  end

  def delivery_error(result)
    return if result.code == 200 || (result.code.zero? && result.message.empty?)

    Error.new(result.message, status_code: result.code)
  end

  def notify_later(task, event, callback)
    complete_callback = proc do |result|
      callback.call(event, result.code, delivery_error(result)&.message)
    rescue StandardError => failure
      @configuration.logger.error("Delivery callback failed: #{failure.class}: #{failure.message}")
    end
    if task.respond_to?(:on_complete)
      task.on_complete(&complete_callback)
    else
      Thread.new do
        result = begin
          task.value
        rescue Error => failure
          Result.new(event: event, code: failure.status_code, message: failure.message)
        end
        complete_callback.call(result)
      end
    end
  end
end
