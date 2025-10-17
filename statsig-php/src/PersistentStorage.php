<?php

namespace Statsig;

use FFI;

abstract class PersistentStorage
{
    public $__ref = null; // phpcs:ignore

    public function __construct()
    {
        $ffi = StatsigFFI::get();

        $this->__ref = $ffi->persistent_storage_create(
            function ($key_ptr, $key_length) {
                $key = $this->convertCCharToString($key_ptr, $key_length);
                $result = $this->load($key);

                if ($result === null) {
                    return null;
                }

                $json = json_encode($result);
                return $this->stringToCChar($json);
            },
            function ($args_ptr, $args_length) {
                $args_json = $this->convertCCharToString($args_ptr, $args_length);
                $args = json_decode($args_json, true);

                if (!isset($args['key'], $args['config_name'], $args['data'])) {
                    return;
                }
                $stickyValues = StickyValues::fromArray($args['data']);
                $this->save($args['key'], $args['config_name'], $stickyValues);
            },
            function ($args_ptr, $args_length) {
                $args_json = $this->convertCCharToString($args_ptr, $args_length);
                $args = json_decode($args_json, true);

                if (!isset($args['key'], $args['config_name'])) {
                    return;
                }

                $this->delete($args['key'], $args['config_name']);
            }
        );
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->persistent_storage_release($this->__ref);
        $this->__ref = null;
    }

    /**
     * Load persisted values for a given key
     *
     * @param string $key Storage key (e.g., "user_id:userID")
     * @return array<string, StickyValues>|null Associative array mapping config names to sticky values
     *                                    (StickyValues as arrays), or null if not found
     * @see StickyValues for the structure of sticky values
     */
    abstract public function load(string $key): ?array;

    /**
     * Save a sticky value for a user and config
     *
     * @param string $key Storage key (e.g., "user_id:userID")
     * @param string $config_name Name of the config/experiment
     * @param StickyValues $data Sticky values to save
     * @return void
     */
    abstract public function save(string $key, string $config_name, StickyValues $data): void;

    /**
     * Delete persisted values for a user and config
     *
     * @param string $key Storage key (e.g., "user_id:userID")
     * @param string $config_name Name of the config/experiment
     * @return void
     */
    abstract public function delete(string $key, string $config_name): void;

    public function getValuesForUser(StatsigUser $user, string $id_type): ?array
    {
        $key = $this->getStorageKey($user, $id_type);
        return $this->load($key);
    }

    private function getStorageKey(StatsigUser $user, string $id_type): string
    {
        $lower_id_type = strtolower($id_type);
        if ($lower_id_type === "user_id" || $lower_id_type === "userid") {
            return $user->user_id . ":userID";
        }
        return  ($user->custom_ids[$id_type] ?? "") . ":" . $id_type;
    }

    private function convertCCharToString($c_char_ptr, int $length): string
    {
        if ($c_char_ptr === null) {
            return '';
        }

        $buffer = FFI::new("char[$length]");
        FFI::memcpy($buffer, $c_char_ptr, $length);
        return FFI::string($buffer, $length);
    }

    private function stringToCChar(string $str)
    {
        $length = strlen($str);
        $buffer = FFI::new("char[$length + 1]", false);
        FFI::memcpy($buffer, $str, $length);
        $buffer[$length] = "\0";
        return $buffer;
    }
}
