<?php

namespace Statsig\Tests;

use Statsig\PersistentStorage;
use Statsig\StickyValues;

class MockPersistentStorage extends PersistentStorage
{
    public $loadCalls = [];
    public $saveCalls = [];
    public $deleteCalls = [];
    private $storage = [];

    public function load(string $key): ?array
    {
        $this->loadCalls[] = $key;
        return $this->storage[$key] ?? null;
    }

    public function save(string $key, string $config_name, StickyValues $data): void
    {
        $this->saveCalls[] = [
            'key' => $key,
            'config_name' => $config_name,
            'data' => $data
        ];

        if (!isset($this->storage[$key])) {
            $this->storage[$key] = [];
        }
        $this->storage[$key][$config_name] = $data->toArray();
    }

    public function delete(string $key, string $config_name): void
    {
        $this->deleteCalls[] = [
            'key' => $key,
            'config_name' => $config_name
        ];

        if (isset($this->storage[$key][$config_name])) {
            unset($this->storage[$key][$config_name]);
        }
    }
}
