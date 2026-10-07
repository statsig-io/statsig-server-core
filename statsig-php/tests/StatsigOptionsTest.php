<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\ProxyConfig;
use Statsig\SpecAdapterConfig;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigLocalFileSpecsAdapter;
use Statsig\StatsigLocalFileEventLoggingAdapter;

class StatsigOptionsTest extends TestCase
{
    public function testCreateAndRelease()
    {
        $options = new StatsigOptions(
            specs_url: "https://statsig.com/specs.json",
            log_event_url: "https://statsig.com/log_event",
            specs_adapter: new StatsigLocalFileSpecsAdapter("", ""),
            event_logging_adapter: new StatsigLocalFileEventLoggingAdapter("", ""),
            environment: "production",
            event_logging_flush_interval_ms: 1000,
            event_logging_max_queue_size: 1000,
            specs_sync_interval_ms: 1000,
            output_log_level: "debug",
            disable_country_lookup: true,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testParitalCreateAndRelease()
    {
        $options = new StatsigOptions(
            environment: "production",
            output_log_level: "debug",
            disable_country_lookup: false,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsInitTimeoutMs()
    {
        $options = new StatsigOptions(
            init_timeout_ms: 5000,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsFallbackToStatsigApi()
    {
        $options = new StatsigOptions(
            fallback_to_statsig_api: true,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsExposureDedupeMaxKeys()
    {
        $options = new StatsigOptions(
            exposure_dedupe_max_keys: 50000,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testExposureDedupeMaxKeysWithOtherOptions()
    {
        $options = new StatsigOptions(
            environment: "staging",
            init_timeout_ms: 2000,
            exposure_dedupe_max_keys: 250000,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsBothNewOptions()
    {
        $options = new StatsigOptions(
            init_timeout_ms: 3000,
            fallback_to_statsig_api: false,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsWithExistingOptions()
    {
        $options = new StatsigOptions(
            environment: "staging",
            output_log_level: "info",
            disable_country_lookup: true,
            init_timeout_ms: 2000,
            fallback_to_statsig_api: true,
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewIdListsOptions()
    {
        $options = new StatsigOptions(
            enable_id_lists: true,
            id_lists_url: "https://custom.statsig.com/id_lists",
            id_lists_sync_interval_ms: 30000
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testIdListsOptionsWithOtherParams()
    {
        $options = new StatsigOptions(
            environment: "staging",
            enable_id_lists: false,
            id_lists_url: "https://test.statsig.com/id_lists",
            id_lists_sync_interval_ms: 15000,
            init_timeout_ms: 5000
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testProxyConfigBasic()
    {
        $proxyConfig = new ProxyConfig("proxy.example.com", 8080);

        $options = new StatsigOptions(
            proxy_config: $proxyConfig
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testProxyConfigWithFullConfig()
    {
        $proxyConfig = new ProxyConfig(
            "proxy.example.com",
            8080,
            "user:password",
            "http"
        );

        $options = new StatsigOptions(
            proxy_config: $proxyConfig
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testProxyConfigWithOtherOptions()
    {
        $proxyConfig = new ProxyConfig("proxy.example.com", 8080);

        $options = new StatsigOptions(
            specs_url: "https://custom.statsig.com/v1/download_config_specs",
            log_event_url: "https://custom.statsig.com/v1/log_event",
            environment: "test",
            init_timeout_ms: 10000,
            proxy_config: $proxyConfig
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testProxyConfigWithAllOptions()
    {
        $proxyConfig = new ProxyConfig(
            "proxy.example.com",
            8080,
            "user:password",
            "http"
        );

        $options = new StatsigOptions(
            specs_url: "https://api.statsig.com",
            log_event_url: "https://events.statsig.com",
            environment: "production",
            init_timeout_ms: 5000,
            disable_network: false,
            disable_all_logging: false,
            proxy_config: $proxyConfig
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testProxyConfigNull()
    {
        $options = new StatsigOptions(
            environment: "staging",
            proxy_config: null
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testSpecAdapterConfigGrpcWithTls()
    {
        $options = new StatsigOptions(
            spec_adapter_config: new SpecAdapterConfig(
                adapterType: SpecAdapterConfig::TYPE_NETWORK_GRPC_WEBSOCKET,
                specsUrl: "https://forward-proxy.example.com",
                initTimeoutMs: 5000,
                authenticationMode: SpecAdapterConfig::AUTH_TLS,
                caCertPath: "/certs/ca.pem",
                clientCertPath: "/certs/client.pem",
                clientKeyPath: "/certs/client-key.pem",
                domainName: "forward-proxy.example.com"
            )
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testSpecAdapterConfigDefaultsInitTimeout()
    {
        $options = new StatsigOptions(
            spec_adapter_config: new SpecAdapterConfig(
                adapterType: SpecAdapterConfig::TYPE_NETWORK_GRPC_WEBSOCKET
            )
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testSpecAdapterConfigWithSpecsUrlAndProxy()
    {
        $options = new StatsigOptions(
            specs_url: "https://custom.statsig.com/v1/download_config_specs",
            proxy_config: new ProxyConfig("proxy.example.com", 8080),
            spec_adapter_config: new SpecAdapterConfig(
                adapterType: SpecAdapterConfig::TYPE_NETWORK_GRPC_WEBSOCKET,
                specsUrl: "http://localhost:50051"
            )
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testHttpSpecAdapterUsesConfiguredUrl()
    {
        $server = new MockServer();
        $statsig = null;

        try {
            $data = file_get_contents(
                dirname(__FILE__) . '/../../statsig-rust/tests/data/eval_proj_dcs.json'
            );
            $server->mock('/proxy-specs/secret-key.json', $data);

            $statsig = new Statsig('secret-key', new StatsigOptions(
                specs_url: $server->getUrl() . '/fallback-specs',
                disable_all_logging: true,
                spec_adapter_config: new SpecAdapterConfig(
                    adapterType: SpecAdapterConfig::TYPE_NETWORK_HTTP,
                    specsUrl: $server->getUrl() . '/proxy-specs'
                )
            ));
            $statsig->initialize();

            $paths = array_column($server->getRequests(), 'path');
            $this->assertContains('/proxy-specs/secret-key.json', $paths);
            $this->assertNotContains('/fallback-specs/secret-key.json', $paths);

            $control = $statsig->getExperimentByGroupName(
                'test_experiment_no_targeting',
                'Control'
            );
            $this->assertEquals('Control', $control->groupName);
            $this->assertEquals(['value' => 'control'], $control->value);
        } finally {
            if ($statsig !== null) {
                $statsig->shutdown();
            }
            $server->stop();
        }
    }

    public function testInvalidProxyPortDoesNotResetOtherOptions()
    {
        $options = new StatsigOptions(
            disable_network: true,
            proxy_config: new ProxyConfig('proxy.example.com', 70000)
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testNegativeSpecAdapterTimeoutDoesNotResetOtherOptions()
    {
        $options = new StatsigOptions(
            disable_network: true,
            spec_adapter_config: new SpecAdapterConfig(
                adapterType: SpecAdapterConfig::TYPE_NETWORK_HTTP,
                specsUrl: 'https://forward-proxy.example.com',
                initTimeoutMs: -1
            )
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testInvalidUtf8EnvironmentStillCreatesOptions()
    {
        $options = new StatsigOptions(
            environment: "prod\xB1",
            disable_network: true
        );
        $this->assertOptionsCreated($options);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    private function assertOptionsCreated(StatsigOptions $options): void
    {
        $this->assertNotNull($options->__ref);
        // PHP FFI returns uint64 refs as signed integers, so a live ref can be
        // negative. Zero is the only failure value.
        $this->assertNotSame('0', sprintf('%u', $options->__ref));
    }
}
