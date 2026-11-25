# frozen_string_literal: true

require 'json'
require_relative 'amplitude_ffi'

class Amplitude
  class Error < StandardError; end

  def initialize(api_key, server_url: nil, timeout_ms: nil)
    config = JSON.generate(api_key: api_key, server_url: server_url, timeout_ms: timeout_ms)
    FFI::MemoryPointer.new(:pointer) do |error_out|
      pointer = Native.amplitude_client_new(config, error_out)
      raise_native_error(error_out.read_pointer)
      @client = Native::Client.new(pointer)
    end
  end

  def track(event_type, user_id: nil, device_id: nil, event_properties: nil, user_properties: nil,
            time: nil, insert_id: nil, session_id: nil)
    event = JSON.generate(
      event_type: event_type,
      user_id: user_id,
      device_id: device_id,
      event_properties: event_properties,
      user_properties: user_properties,
      time: time,
      insert_id: insert_id,
      session_id: session_id
    )
    raise_native_error(Native.amplitude_client_track(@client, event))
  end

  private

  def raise_native_error(error)
    return if error.null?

    begin
      raise Error, error.read_string.force_encoding(Encoding::UTF_8)
    ensure
      Native.amplitude_error_free(error)
    end
  end
end
