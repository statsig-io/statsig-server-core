<?php

namespace Statsig;

use FFI;

class StatsigFFI
{
    private static ?FFI $ffi = null;

    public static function get(): FFI
    {
        if (self::$ffi !== null) {
            return self::$ffi;
        }

        $found_binary_path = null;
        $found_header_path = null;

        $bin = dirname(__FILE__) . '/../resources';
        if (is_dir($bin)) {
            $found_binary_path = self::find_binary_in_dir($bin);
            $found_header_path = self::find_header_file_in_dir($bin);
        }

        if ($found_binary_path === null) {
            $target_dir = dirname(__FILE__) . '/../../target/debug';
            if (is_dir($target_dir)) {
                $found_binary_path = self::find_binary_in_dir($target_dir);
            }
        }

        if ($found_header_path === null) {
            $include_dir = dirname(__FILE__) . '/../../statsig-ffi/include';
            if (is_dir($include_dir)) {
                $found_header_path = self::find_header_file_in_dir($include_dir);
            }
        }


        if ($found_binary_path === null) {
            error_log("Binary not found in $bin\n");
        }

        if ($found_header_path === null) {
            error_log("Header file not found in $include_dir\n");
        }

        self::$ffi = FFI::cdef(
            file_get_contents($found_header_path),
            $found_binary_path
        );

        return self::$ffi;
    }

    private static function find_binary_in_dir(string $dir): ?string
    {
        $file_name = match (PHP_OS_FAMILY) {
            'Darwin' => 'libstatsig_ffi.dylib',
            'Windows' => 'statsig_ffi.dll',
            default => 'libstatsig_ffi.so',
        };

        $path = $dir . '/' . $file_name;

        if (file_exists($path)) {
            return $path;
        }

        return null;
    }

    private static function find_header_file_in_dir(string $dir): ?string
    {
        $path = $dir . '/statsig_ffi.h';

        if (file_exists($path)) {
            return $path;
        }

        return null;
    }
}
