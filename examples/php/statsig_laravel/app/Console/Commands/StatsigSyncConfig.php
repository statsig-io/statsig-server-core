<?php

namespace App\Console\Commands;

use Illuminate\Console\Command;
use Illuminate\Support\Facades\Log;
use Statsig\StatsigLocalFileSpecsAdapter;

class StatsigSyncConfig extends Command
{
    /**
     * The name and signature of the console command.
     *
     * @var string
     */
    protected $signature = 'statsig:sync-configuration';

    /**
     * The console command description.
     *
     * @var string
     */
    protected $description = 'Pulls the latest configuration from Statsig';

    /**
     * Execute the console command.
     */
    public function handle()
    {
        Log::debug("Syncing Statsig configuration...");

        $adapter = new StatsigLocalFileSpecsAdapter(env("STATSIG_SECRET_KEY"), "/tmp");
        $adapter->sync_specs_from_network();

        Log::debug("Statsig configuration synced");
    }
}
