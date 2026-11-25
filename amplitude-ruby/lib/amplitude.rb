# frozen_string_literal: true

require 'json'
require_relative 'amplitude_ffi'
require_relative 'amplitude/identify'

class Amplitude
  class Error < StandardError; end

  class FlushTask
    def wait
      raise NotImplementedError, 'Not implemented'
    end
  end

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

  def configuration
    not_implemented
  end

  def configuration=(_configuration)
    not_implemented
  end

  def api_key
    not_implemented
  end

  def api_key=(_api_key)
    not_implemented
  end

  def track(event = {}, **fields)
    payload = event.is_a?(Hash) ? event.transform_keys(&:to_s) : { 'event_type' => event }
    payload.merge!(fields.transform_keys(&:to_s))
    options = payload.delete('options') || {}
    request = JSON.generate(options.transform_keys(&:to_s).merge(payload))
    raise_native_error(Native.amplitude_client_track(@client, request))
  end

  def identify(operations, options = nil, event_properties: nil, **option_overrides)
    request = JSON.generate(
      operations: operations.to_a,
      options: (options || {}).merge(option_overrides),
      event_properties: event_properties
    )
    raise_native_error(Native.amplitude_client_identify(@client, request))
  end

  def group_identify(_group_type, _group, _operations, options: nil, event_properties: nil, user_properties: nil)
    not_implemented
  end

  def revenue(_transaction, _options = nil, **_option_overrides)
    not_implemented
  end

  def set_group(_group_type, _group, _options = nil, **_option_overrides)
    not_implemented
  end

  def flush
    not_implemented
  end

  def add(_plugin, _stage)
    not_implemented
  end

  def remove(_plugin)
    not_implemented
  end

  def shutdown
    not_implemented
  end

  private

  def not_implemented
    raise NotImplementedError, 'Not implemented'
  end

  def raise_native_error(error)
    return if error.null?

    begin
      raise Error, error.read_string.force_encoding(Encoding::UTF_8)
    ensure
      Native.amplitude_error_free(error)
    end
  end
end
