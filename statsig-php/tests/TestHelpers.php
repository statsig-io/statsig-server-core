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

    public static function ensureEmptyDir($dir): void
    {
        if (is_dir($dir)) {
            $files = glob($dir . '/*');
            foreach ($files as $file) {
                if (is_file($file)) {
                    unlink($file);
                }
            }
            rmdir($dir);
        }
        mkdir($dir);
    }
}
