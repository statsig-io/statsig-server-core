# frozen_string_literal: true

require_relative 'amplitude/payload'
require_relative 'amplitude/identify'
require_relative 'amplitude/delivery'

class Amplitude
  class Error < StandardError
    attr_reader :status_code

    def initialize(message = nil, status_code: 0)
      super(message)
      @status_code = status_code
    end
  end

  def initialize(api_key, configuration = nil, **configuration_overrides)
    @delivery = Delivery.new(api_key, merge_options(configuration, configuration_overrides))
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
    complete(event, callback || block) do
      event['options'] = merge_options(options, option_overrides)
      event.replace(Payload.event(event))
      begin
        event['user_properties'] = Identify.user_properties(operations)
      rescue Error
        raise unless @delivery.opt_out
      end
    end
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
      if @delivery.opt_out
        0
      else
        Payload.validate_event(event)
        code, message = @delivery.submit(event)
        raise Error.new(message, status_code: code) if message

        code
      end
    rescue Error => failure
      error = failure
      failure.status_code
    end
    callback&.call(event, status, error&.message)
    raise error if error

    nil
  end
end
