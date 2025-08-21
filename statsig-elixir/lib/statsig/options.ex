defmodule Statsig.Options do
  defstruct environment: nil,
            output_log_level: nil,
            init_timeout_ms: nil,
            fallback_to_statsig_api: nil,
            event_logging_flush_interval_ms: nil,
            event_logging_max_queue_size: nil,
            log_event_url: nil,
            specs_sync_interval_ms: nil,
            specs_url: nil,
            enable_id_lists: nil,
            id_lists_url: nil,
            id_lists_sync_interval_ms: nil,
            wait_for_country_lookup_init: nil,
            wait_for_user_agent_init: nil,
            disable_all_logging: nil,
            disable_country_lookup: nil,
            disable_network: nil,
            disable_user_agent_parsing: nil
end

defmodule Statsig.ExperimentEvaluationOptions do
  defstruct disable_exposure_logging: false
end

defmodule Statsig.FeatureGateEvaluationOptions do
  defstruct disable_exposure_logging: false
end

defmodule Statsig.LayerEvaluationOptions do
  defstruct disable_exposure_logging: false
end

defmodule Statsig.DynamicConfigEvaluationOptions do
  defstruct disable_exposure_logging: false
end

defmodule Statsig.ClientInitResponseOptions do
  defstruct hash_algorithm: nil,
            client_sdk_key: nil,
            include_local_overrides: nil
end
