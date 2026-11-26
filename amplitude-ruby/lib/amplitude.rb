# frozen_string_literal: true

require 'json'
require_relative 'amplitude_ffi'
require_relative 'amplitude/identify'

class Amplitude
  class Error < StandardError; end

  EVENT_FIELDS = %w[event_type event_properties user_properties groups group_properties library].freeze
  private_constant :EVENT_FIELDS

  def initialize(api_key, configuration = nil, **configuration_overrides)
    config = JSON.generate(
      api_key: api_key,
      configuration: (configuration || {}).merge(configuration_overrides)
    )
    FFI::MemoryPointer.new(:pointer) do |error_out|
      pointer = Native.amplitude_client_new(config, error_out)
      raise_native_error(error_out.read_pointer)
      @client = Native::Client.new(pointer)
    end
  end

  def track(event = {}, callback: nil, **fields, &block)
    payload = event.is_a?(Hash) ? event.transform_keys(&:to_s) : { 'event_type' => event }
    payload.merge!(fields.transform_keys(&:to_s))
    options = payload.delete('options') || {}
    normalized = payload.slice(*EVENT_FIELDS)
    normalized['options'] = options.transform_keys(&:to_s).merge(payload.except(*EVENT_FIELDS))
    deliver(:amplitude_client_track_with_completion, normalized, callback || block, normalized)
  end

  def identify(operations, options = nil, event_properties: nil, callback: nil, **option_overrides, &block)
    request = {
      operations: operations.to_a,
      options: (options || {}).transform_keys(&:to_s).merge(option_overrides.transform_keys(&:to_s)),
      event_properties: event_properties
    }
    event = { 'event_type' => '$identify', 'options' => request[:options] }
    event['event_properties'] = event_properties if event_properties
    deliver(:amplitude_client_identify_with_completion, request, callback || block, event)
  end

  private

  def raise_native_error(error)
    message = read_native_string(error)
    raise Error, message if message
  end

  def deliver(operation, payload, callback, original_event)
    json = JSON.generate(payload)
    unless callback
      return raise_native_error(Native.public_send(operation, @client, json, nil))
    end

    FFI::MemoryPointer.new(:pointer) do |completion_out|
      error = read_native_string(Native.public_send(operation, @client, json, completion_out))
      completion = read_native_string(completion_out.read_pointer)
      if completion
        result = JSON.parse(completion)
        callback.call(result.fetch('event'), result.fetch('status_code'), result['message'])
      else
        callback.call(original_event, 0, error)
      end
      raise Error, error if error
    end
    nil
  end

  def read_native_string(pointer)
    return if pointer.null?

    begin
      pointer.read_string.force_encoding(Encoding::UTF_8)
    ensure
      Native.amplitude_string_free(pointer)
    end
  end
end
