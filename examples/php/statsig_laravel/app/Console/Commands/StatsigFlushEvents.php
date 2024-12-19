<?php

namespace App\Console\Commands;

use Illuminate\Console\Command;
use Illuminate\Support\Facades\Log;
use Statsig\StatsigLocalFileEventLoggingAdapter;

class StatsigFlushEvents extends Command
{
    /**
     * The name and signature of the console command.
     *
     * @var string
     */
    protected $signature = 'statsig:flush-events';

    /**
     * The console command description.
     *
     * @var string
     */
    protected $description = 'Sends any pending events to Statsig';

    /**
     * Execute the console command.
     */
    public function handle()
    {
        Log::debug("Flushing Statsig events...");

        $adapter = new StatsigLocalFileEventLoggingAdapter(env("STATSIG_SECRET_KEY"), "/tmp", "https://beyondwickedmapping.org/v1/rgstr");
        $adapter->sendPendingEvents();

        Log::debug("Statsig events flushed");
    }
}
