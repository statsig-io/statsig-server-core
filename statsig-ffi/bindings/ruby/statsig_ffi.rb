require 'ffi'

module StatsigFFI
  extend FFI::Library
  ffi_lib File.expand_path('./libstatsig_ffi.dylib', __dir__)

  class User < FFI::Struct
    layout :name, :string,
           :email, :string
  end

  attach_function :create_user, [:string, :string], :pointer
  attach_function :destroy_user, [:pointer], :void
  attach_function :create_statsig_for_user, [:pointer, :string], :pointer
  attach_function :destroy_statsig, [:pointer], :void
  attach_function :check_gate, [:pointer, :string], :bool
  attach_function :statsig_get_client_init_response, [:pointer], :string
end


