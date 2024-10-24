<?php

namespace Statsig;

use FFI;

class StatsigFFI
{
    private static ?FFI $ffi = null;
    public static function get(): FFI
    {
        if (self::$ffi === null) {
            $dir = dirname(__FILE__);
            self::$ffi = FFI::cdef(
                file_get_contents($dir . '/../../statsig-ffi/include/statsig_ffi.h'),
                match (PHP_OS_FAMILY) {
                    'Darwin' => $dir . '/../../target/debug/libstatsig_ffi.dylib',
                    'Windows' => $dir . '/../../target/debug/statsig_ffi.dll',
                    default => $dir . '/../../target/debug/libstatsig_ffi.so',
                }
            );
        }
        return self::$ffi;
    }
}
