<?php

namespace Statsig\Tests;

use Statsig\SpecsAdapterBase;
use Statsig\SpecsUpdateListener;

class MockSpecsAdapter extends SpecsAdapterBase
{
    public $setup_called = false;
    public $start_called = false;
    public $shutdown_called = false;
    public $schedule_background_sync_called = false;

    public ?SpecsUpdateListener $listener = null;

    public function setup(SpecsUpdateListener $listener)
    {
        $this->listener = $listener;
    }

    public function start()
    {
        $timestamp = intval(microtime(true) * 1000);
        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/eval_proj_dcs.json');

        $this->listener->didReceiveSpecsUpdate($data, "Mock", $timestamp);
    }

    public function shutdown()
    {
        $this->shutdown_called = true;
    }

    public function scheduleBackgroundSync()
    {
        $this->schedule_background_sync_called = true;
    }
}
