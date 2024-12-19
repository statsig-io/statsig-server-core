<?php

use App\Console\Commands\StatsigFlushEvents;
use App\Console\Commands\StatsigSyncConfig;
use Illuminate\Support\Facades\Schedule;


Schedule::command(StatsigFlushEvents::class)->everyTenSeconds();
Schedule::command(StatsigSyncConfig::class)->everyMinute();
