<?php
namespace Statsig\Tests;
abstract class TestHelpers
{
    public static function waitUntilTrue($context, $callback, $timeout_secs = 1.0): void
    {
        $start = microtime(true);
        while (!$callback() && microtime(true) - $start < $timeout_secs) {
            usleep(10000); // Sleep for 10ms
        }

        if (!$callback()) {
            $context->fail("Timed out waiting for callback");
        }
    }
}
