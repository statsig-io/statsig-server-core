<?php

namespace Statsig\StatsigFFI;

use FFI;

class StatsigFFI
{
    private static ?FFI $ffi = null;
    public static function get()
    {
        if (self::$ffi === null) {
            self::$ffi = FFI::cdef(
                file_get_contents(__DIR__ . '/../../../include/statsig_ffi.h'),
                __DIR__ . '/../../../../target/release/libstatsig_ffi.dylib'
            );
        }
        return self::$ffi;
    }
}
