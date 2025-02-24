defmodule Statsig do
  use GenServer

  def start_link(sdk_key, options) do
    GenServer.start_link(__MODULE__, {sdk_key, options}, name: __MODULE__)
  end

  def init({sdk_key, statsig_options}) do
    instance = NativeBindings.new(sdk_key, statsig_options)
    {:ok, instance}
  end

  def handle_call(:get_instance, _from, state) do
    {:reply, state, state}
  end

  def get_statsig_instance do
    GenServer.call(__MODULE__, :get_instance)
  end

  def initialize() do
    instance = get_statsig_instance()
    NativeBindings.initialize(instance)
  end

  def check_gate(gate_name, statsig_user) do
    instance = get_statsig_instance()
    NativeBindings.check_gate(instance, gate_name, statsig_user)
  end

  def get_feature_gate(gate_name, statsig_user) do
    instance = get_statsig_instance()
    NativeBindings.get_feature_gate(instance, gate_name, statsig_user)
  end

  def get_config(config_name, statsig_user) do
    instance = get_statsig_instance()
    NativeBindings.get_config(instance, config_name, statsig_user)
  end

  def get_experiment(experiment_name, statsig_user) do
    instance = get_statsig_instance()
    NativeBindings.get_experiment(instance, experiment_name, statsig_user)
  end

  def get_layer(layer_name, statsig_user) do
    instance = get_statsig_instance()
    NativeBindings.get_layer(instance, layer_name, statsig_user)
  end

  def log_event(statsig_user, event_name, value, metadata) do
    instance = get_statsig_instance()
    NativeBindings.log_event(instance, statsig_user, event_name, value, metadata)
  end

  def flush() do
    instance = get_statsig_instance()
    NativeBindings.flush(instance)
  end

  def shutdown() do
    instance = get_statsig_instance()
    NativeBindings.shutdown(instance)
  end

end
