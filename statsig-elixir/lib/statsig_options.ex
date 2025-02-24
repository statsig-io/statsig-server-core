
defmodule StatsigOptions do
  defstruct [
    environment: nil,
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
  ]
end
