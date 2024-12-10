<?php

use App\Console\Commands\StatsigFlushEvents;
use App\Console\Commands\StatsigSyncConfig;
use Illuminate\Support\Facades\Schedule;


Schedule::command(StatsigFlushEvents::class)->everyMinute();
Schedule::command(StatsigSyncConfig::class)->everyMinute();
