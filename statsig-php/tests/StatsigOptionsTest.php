<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\ProxyConfig;
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
        $this->assertNotNull($options->__ref);

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
        $this->assertNotNull($options->__ref);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsInitTimeoutMs()
    {
        $options = new StatsigOptions(
            init_timeout_ms: 5000,
        );
        $this->assertNotNull($options->__ref);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsFallbackToStatsigApi()
    {
        $options = new StatsigOptions(
            fallback_to_statsig_api: true,
        );
        $this->assertNotNull($options->__ref);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }

    public function testNewOptionsBothNewOptions()
    {
        $options = new StatsigOptions(
            init_timeout_ms: 3000,
            fallback_to_statsig_api: false,
        );
        $this->assertNotNull($options->__ref);

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
        $this->assertNotNull($options->__ref);

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
        $this->assertNotNull($options->__ref);

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
        $this->assertNotNull($options->__ref);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testProxyConfigBasic()
    {
        $proxyConfig = new ProxyConfig("proxy.example.com", 8080);

        $options = new StatsigOptions(
            proxy_config: $proxyConfig
        );
        $this->assertNotNull($options->__ref);

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
        $this->assertNotNull($options->__ref);

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
        $this->assertNotNull($options->__ref);

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
        $this->assertNotNull($options->__ref);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }

    public function testProxyConfigNull()
    {
        $options = new StatsigOptions(
            environment: "staging",
            proxy_config: null
        );
        $this->assertNotNull($options->__ref);

        $options->__destruct();
        $this->assertNull($options->__ref);
    }
}
