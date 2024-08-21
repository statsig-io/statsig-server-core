require_relative 'statsig_ffi'

class User
  def initialize(name, email)
    @user_ptr = StatsigFFI.create_user(name, email)
  end

  def self.create(name, email)
    new(name, email)
  end

  def destroy
    StatsigFFI.destroy_user(@user_ptr) if @user_ptr
  end

  def self.destroy(user)
    user.destroy
  end
end

class Statsig
  def initialize(user, sdk_key)
    @statsig_ptr = StatsigFFI.create_statsig_for_user(user.instance_variable_get(:@user_ptr), sdk_key)
  end

  def self.for_user(user, sdk_key)
    new(user, sdk_key)
  end

  def destroy
    StatsigFFI.destroy_statsig(@statsig_ptr) if @statsig_ptr
  end

  def self.destroy(statsig)
    statsig.destroy
  end

  def check_gate(gate_name)
    StatsigFFI.check_gate(@statsig_ptr, gate_name)
  end

  def get_client_init_res
    StatsigFFI.statsig_get_client_init_response(@statsig_ptr)
  end
end
