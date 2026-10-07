<?php

namespace Statsig;

class StatsigOptions
{
    public $__ref = null; // phpcs:ignore

    /**
     * @param int|null $event_logging_flush_interval_ms Deprecated. Accepted for
     *        compatibility. Core ignores this value, so it is not sent.
     */
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
        ?bool $wait_for_country_lookup_init = null,
        ?bool $wait_for_user_agent_init = null,
        ?bool $enable_id_lists = null,
        ?bool $enable_dcs_deltas = null,
        ?bool $disable_network = null,
        ?string $id_lists_url = null,
        ?int $id_lists_sync_interval_ms = null,
        ?bool $disable_all_logging = null,
        ?int $init_timeout_ms = null,
        ?bool $fallback_to_statsig_api = null,
        ?bool $use_third_party_ua_parser = null,
        ?PersistentStorage $persistent_storage = null,
        ?ProxyConfig $proxy_config = null,
        ?int $id_lists_request_timeout_ms = null,
        ?int $exposure_dedupe_max_keys = null,
        ?SpecAdapterConfig $spec_adapter_config = null
    ) {
        $data = [];

        self::assignIfSet($data, 'specs_url', $specs_url);
        self::assignIfSet($data, 'log_event_url', $log_event_url);
        self::assignRef($data, 'specs_adapter_ref', $specs_adapter);
        self::assignRef($data, 'event_logging_adapter_ref', $event_logging_adapter);
        self::assignIfSet($data, 'environment', $environment);
        self::assignIfSet($data, 'event_logging_max_queue_size', $event_logging_max_queue_size);
        self::assignIfSet($data, 'specs_sync_interval_ms', $specs_sync_interval_ms);
        self::assignIfSet($data, 'output_log_level', $output_log_level);
        self::assignIfSet($data, 'disable_country_lookup', $disable_country_lookup);
        self::assignIfSet($data, 'wait_for_country_lookup_init', $wait_for_country_lookup_init);
        self::assignIfSet($data, 'wait_for_user_agent_init', $wait_for_user_agent_init);
        self::assignIfSet($data, 'enable_id_lists', $enable_id_lists);
        self::assignIfSet($data, 'enable_dcs_deltas', $enable_dcs_deltas);
        self::assignIfSet($data, 'disable_network', $disable_network);
        self::assignIfSet($data, 'id_lists_url', $id_lists_url);
        self::assignIfSet($data, 'id_lists_sync_interval_ms', $id_lists_sync_interval_ms);
        self::assignIfSet($data, 'disable_all_logging', $disable_all_logging);
        self::assignIfSet($data, 'init_timeout_ms', $init_timeout_ms);
        self::assignIfSet($data, 'fallback_to_statsig_api', $fallback_to_statsig_api);
        self::assignIfSet($data, 'use_third_party_ua_parser', $use_third_party_ua_parser);
        self::assignRef($data, 'persistent_storage_ref', $persistent_storage);
        self::assignIfSet($data, 'id_lists_request_timeout_ms', $id_lists_request_timeout_ms);
        self::assignIfSet($data, 'exposure_dedupe_max_keys', $exposure_dedupe_max_keys);

        if ($proxy_config !== null) {
            self::assignIfSet($data, 'proxy_host', $proxy_config->proxyHost);
            if ($proxy_config->proxyPort >= 1 && $proxy_config->proxyPort <= 65535) {
                $data['proxy_port'] = $proxy_config->proxyPort;
            }
            self::assignIfSet($data, 'proxy_auth', $proxy_config->proxyAuth);
            self::assignIfSet($data, 'proxy_protocol', $proxy_config->proxyProtocol);
        }

        if ($spec_adapter_config !== null) {
            self::assignIfSet($data, 'spec_adapter_type', $spec_adapter_config->adapterType);
            $initTimeoutMs = $spec_adapter_config->initTimeoutMs
                ?? SpecAdapterConfig::DEFAULT_INIT_TIMEOUT_MS;
            self::assignIfSet($data, 'spec_adapter_init_timeout_ms', $initTimeoutMs);
            self::assignIfSet($data, 'spec_adapter_url', $spec_adapter_config->specsUrl);
            self::assignIfSet(
                $data,
                'spec_adapter_authentication_mode',
                $spec_adapter_config->authenticationMode
            );
            self::assignIfSet($data, 'spec_adapter_ca_cert_path', $spec_adapter_config->caCertPath);
            self::assignIfSet(
                $data,
                'spec_adapter_client_cert_path',
                $spec_adapter_config->clientCertPath
            );
            self::assignIfSet(
                $data,
                'spec_adapter_client_key_path',
                $spec_adapter_config->clientKeyPath
            );
            self::assignIfSet($data, 'spec_adapter_domain_name', $spec_adapter_config->domainName);
        }

        $json = self::encodeOptionsData($data);
        if ($json === null) {
            error_log("StatsigOptions failed to encode; falling back to default options\n");
            $this->__ref = 0;
            return;
        }

        $this->__ref = StatsigFFI::get()->statsig_options_create_from_data($json);
        if (sprintf('%u', $this->__ref) === '0') {
            error_log("StatsigOptions failed to initialize; falling back to default options\n");
        }
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_options_release($this->__ref);
        $this->__ref = null;
    }

    /**
     * @param array<string, mixed> $data
     */
    private static function encodeOptionsData(array $data): ?string
    {
        $json = json_encode(
            $data !== [] ? $data : new \stdClass(),
            JSON_INVALID_UTF8_SUBSTITUTE
        );
        if ($json === false) {
            return null;
        }

        // Refs are unsigned 64-bit values. PHP stores them as signed integers
        // or their decimal strings, so they are encoded as strings and then
        // rewritten as JSON numbers. A negative number would fail to
        // deserialize into the core's u64 fields.
        $json = preg_replace(
            '/"(specs_adapter_ref|event_logging_adapter_ref|persistent_storage_ref)":"(\d+)"/',
            '"$1":$2',
            $json
        );

        return is_string($json) ? $json : null;
    }

    /**
     * @param array<string, mixed> $data
     * @param mixed $value
     */
    private static function assignIfSet(array &$data, string $key, $value): void
    {
        if ($value === null) {
            return;
        }

        if (is_string($value) && $value === '') {
            return;
        }

        if (is_int($value) && $value < 0) {
            return;
        }

        $data[$key] = $value;
    }

    /**
     * @param array<string, mixed> $data
     */
    private static function assignRef(array &$data, string $key, ?object $holder): void
    {
        if ($holder === null || !isset($holder->__ref)) {
            return;
        }

        $ref = sprintf('%u', $holder->__ref);
        if ($ref === '0') {
            return;
        }

        $data[$key] = $ref;
    }
}
