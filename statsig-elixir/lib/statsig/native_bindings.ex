defmodule Statsig.NativeBindings do
  version = Mix.Project.config()[:version] |> to_string()
  use RustlerPrecompiled,
    otp_app: :statsig_elixir,
    crate: "statsig_elixir",
    version: version,
    base_url: "https://github.com/statsig-io/statsig-elixir-core/releases/download/#{version}/",
    force_build: System.get_env("FORCE_STATSIG_NATIVE_BUILD") in ["1", "true"],
    targets: [
      "aarch64-apple-darwin", # Add other supported targets if needed
      "aarch64-unknown-linux-gnu",
      "x86_64-apple-darwin",
      "x86_64-unknown-linux-gnu",
      "x86_64-unknown-linux-musl",
      "aarch64-unknown-linux-musl",
    ]
  def new(_key, _options, _system_metadata), do: :erlang.nif_error(:nif_not_loaded)
  def initialize(_statsig), do: :erlang.nif_error(:nif_not_loaded)
  def check_gate(_statsig, _gate_name, _statsig_user, _options), do: :erlang.nif_error(:nif_not_loaded)
  def get_feature_gate(_statsig, _gate_name, _statsig_user, _options), do: :erlang.nif_error(:nif_not_loaded)
  def get_dynamic_config(_statsig, _config_name, _statsig_user, _options), do: :erlang.nif_error(:nif_not_loaded)
  def get_experiment(_statsig, _experiment_name, _statsig_user, _options), do: :erlang.nif_error(:nif_not_loaded)
  def get_layer(_statsig, _layer_name, _statsig_user, _options), do: :erlang.nif_error(:nif_not_loaded)
  def get_prompt(_statsig, _prompt_name, _statsig_user, _options), do: :erlang.nif_error(:nif_not_loaded)
  def get_client_init_response_as_string(_statsig, _statsig_user, _options), do: :erlang.nif_error(:nif_not_loaded)
  def log_event(_statsig, _statsig_user, _event_name,_value, _metadata), do: :erlang.nif_error(:nif_not_loaded)
  def log_event_with_number(_statsig, _statsig_user, _event_name,_value, _metadata), do: :erlang.nif_error(:nif_not_loaded)
  def flush(_statsig), do: :erlang.nif_error(:nif_not_loaded)
  def shutdown(_statsig), do: :erlang.nif_error(:nif_not_loaded)

  # Layer Related Functions
  def layer_get_name(_layer), do: :erlang.nif_error(:nif_not_loaded)
  def layer_get_rule_id(_layer), do: :erlang.nif_error(:nif_not_loaded)
  def layer_get(_layer, _param_name, _default_value), do: :erlang.nif_error(:nif_not_loaded)
  def layer_get_group_name(_layer), do: :erlang.nif_error(:nif_not_loaded)

end
