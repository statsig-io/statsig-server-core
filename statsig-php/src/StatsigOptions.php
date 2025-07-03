<?php

namespace Statsig;

const IGNORED_ENABLE_ID_LISTS = -1;
const IGNORED_ID_LISTS_URL = null;
const IGNORED_ID_LISTS_SYNC_INTERVAL_MS = -1;

class StatsigOptions
{
    public $__ref = null; // phpcs:ignore

    public function __construct(
        ?string $specs_url = null,
        ?string $log_event_url = null,
        ?object $specs_adapter = null,
        ?object $event_logging_adapter = null,
        ?string $environment = null,
        ?int $event_logging_flush_interval_ms = null,
        ?int $event_logging_max_queue_size = null,
        ?int $specs_sync_interval_ms = null,
        ?string $output_log_level = null,
        ?bool $disable_country_lookup = null,
        ?bool $disable_user_agent_parsing = null,
        ?bool $wait_for_country_lookup_init = null,
        ?bool $wait_for_user_agent_init = null,
        ?bool $disable_all_logging = null
    ) {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_options_create(
            $specs_url,
            $log_event_url,
            is_null($specs_adapter) ? 0 : $specs_adapter->__ref,
            is_null($event_logging_adapter) ? 0 : $event_logging_adapter->__ref,
            $environment,
            $event_logging_flush_interval_ms ?? -1,
            $event_logging_max_queue_size ?? -1,
            $specs_sync_interval_ms ?? -1,
            $output_log_level ?? null,
            toSafeOptBool($disable_country_lookup),
            toSafeOptBool($disable_user_agent_parsing),
            toSafeOptBool($wait_for_country_lookup_init),
            toSafeOptBool($wait_for_user_agent_init),
            IGNORED_ENABLE_ID_LISTS,
            IGNORED_ID_LISTS_URL,
            IGNORED_ID_LISTS_SYNC_INTERVAL_MS,
            toSafeOptBool($disable_all_logging),
            null, // global custom fields
        );
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_options_release($this->__ref);
        $this->__ref = null;
    }
}

function toSafeOptBool(?bool $value): int
{
    if (is_null($value)) {
        return -1;
    }

    return $value ? 1 : 0;
}
