<?php

namespace App\Providers;

use Illuminate\Contracts\Foundation\Application;
use Illuminate\Contracts\Support\DeferrableProvider;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\ServiceProvider;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;
use Statsig\StatsigLocalFileEventLoggingAdapter;
use Statsig\StatsigLocalFileSpecsAdapter;

class StatsigProvider extends ServiceProvider implements DeferrableProvider
{
    public function register(): void
    {
        $this->app->singleton(Statsig::class, function (Application $app) {
            $sdk_key = env("STATSIG_SECRET_KEY");

            $options = new StatsigOptions(
                null,
                null,
                new StatsigLocalFileSpecsAdapter($sdk_key, "/tmp"),
                new StatsigLocalFileEventLoggingAdapter($sdk_key, "/tmp")
            );

            Log::debug("Creating Statsig Instance");

            return new Statsig($sdk_key, $options);
        });
    }

    public function boot(): void
    {
        $statsig = $this->app->make(Statsig::class);
        $statsig->initialize();
    }

    public function provides(): array
    {
        return [Statsig::class];
    }
}
