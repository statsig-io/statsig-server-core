defmodule NativeBindings do
  use Rustler,
    otp_app: :statsig,
    crate: "statsig_elixir"
  def new(_key, _options), do: :erlang.nif_error(:nif_not_loaded)
  def initialize(_statsig), do: :erlang.nif_error(:nif_not_loaded)
  def check_gate(_statsig, _gate_name, _statsig_user), do: :erlang.nif_error(:nif_not_loaded)
  def get_feature_gate(_statsig, _gate_name, _statsig_user), do: :erlang.nif_error(:nif_not_loaded)
  def get_config(_statsig, _config_name, _statsig_user), do: :erlang.nif_error(:nif_not_loaded)
  def get_experiment(_statsig, _experiment_name, _statsig_user), do: :erlang.nif_error(:nif_not_loaded)
  def get_layer(_statsig, _layer_name, _statsig_user), do: :erlang.nif_error(:nif_not_loaded)
  def log_event(_statsig, _statsig_user, _event_name,_value, _metadata), do: :erlang.nif_error(:nif_not_loaded)
  def flush(_statsig), do: :erlang.nif_error(:nif_not_loaded)
  def shutdown(_statsig), do: :erlang.nif_error(:nif_not_loaded)

  # Layer Related Functions
  def layer_get_name(_layer), do: :erlang.nif_error(:nif_not_loaded)
  def layer_get_rule_id(_layer), do: :erlang.nif_error(:nif_not_loaded)
  def layer_get(_layer, _param_name, _default_value), do: :erlang.nif_error(:nif_not_loaded)
  def layer_get_group_name(_layer), do: :erlang.nif_error(:nif_not_loaded)

end
