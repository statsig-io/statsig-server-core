<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
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
            disable_user_agent_parsing: false,
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
            disable_user_agent_parsing: false,
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
            disable_user_agent_parsing: false,
            init_timeout_ms: 2000,
            fallback_to_statsig_api: true,
        );
        $this->assertNotNull($options->__ref);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }
}
