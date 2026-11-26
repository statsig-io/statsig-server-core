# frozen_string_literal: true

require 'ffi'

class Amplitude
  module Native
    extend FFI::Library

    library = FFI.map_library_name('amplitude_ruby_ffi')
    ffi_lib ENV.fetch('AMPLITUDE_LIB_PATH') { File.expand_path("../../target/debug/#{library}", __dir__) }

    attach_function :amplitude_client_new, %i[string pointer], :pointer
    attach_function :amplitude_client_track_with_completion, %i[pointer string pointer], :pointer, blocking: true
    attach_function :amplitude_client_identify_with_completion, %i[pointer string pointer], :pointer, blocking: true
    attach_function :amplitude_client_free, [:pointer], :void
    attach_function :amplitude_string_free, [:pointer], :void

    class Client < FFI::AutoPointer
      def self.release(pointer)
        Native.amplitude_client_free(pointer)
      end
    end
  end

  private_constant :Native
end
