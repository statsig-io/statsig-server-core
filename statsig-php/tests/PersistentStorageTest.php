<?php

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigOptions;
use Statsig\Tests\MockPersistentStorage;

class PersistentStorageTest extends TestCase
{
    public function testPersistentStorageCreate()
    {
        $storage = new MockPersistentStorage();
        $this->assertNotNull($storage->__ref);
        $this->assertNotEquals(0, $storage->__ref);
    }

    public function testPersistentStorageLoad()
    {
        $storage = new MockPersistentStorage();

        // Test load through FFI
        $result = \Statsig\StatsigFFI::get()->__internal__test_persistent_storage(
            $storage->__ref,
            'load',
            'user_123:userID',
            '',
            ''
        );

        $this->assertCount(1, $storage->loadCalls);
        $this->assertEquals('user_123:userID', $storage->loadCalls[0]);
    }

    public function testPersistentStorageSave()
    {
        $storage = new MockPersistentStorage();

        $testData = json_encode([
            'value' => true,
            'json_value' => null,
            'rule_id' => 'test_rule',
            'group_name' => 'test_group',
            'secondary_exposures' => [],
            'undelegated_secondary_exposures' => null,
            'config_delegate' => null,
            'explicit_parameters' => null,
            'time' => 1234567890,
            'config_version' => 1
        ]);

        // Test save through FFI
        \Statsig\StatsigFFI::get()->__internal__test_persistent_storage(
            $storage->__ref,
            'save',
            'user_123:userID',
            'test_experiment',
            $testData
        );

        $this->assertCount(1, $storage->saveCalls);
        $this->assertEquals('user_123:userID', $storage->saveCalls[0]['key']);
        $this->assertEquals('test_experiment', $storage->saveCalls[0]['config_name']);
        $this->assertInstanceOf(\Statsig\StickyValues::class, $storage->saveCalls[0]['data']);
        $this->assertTrue($storage->saveCalls[0]['data']->value);
    }

    public function testPersistentStorageDelete()
    {
        $storage = new MockPersistentStorage();

        // Test delete through FFI
        \Statsig\StatsigFFI::get()->__internal__test_persistent_storage(
            $storage->__ref,
            'delete',
            'user_123:userID',
            'test_experiment',
            ''
        );

        $this->assertCount(1, $storage->deleteCalls);
        $this->assertEquals('user_123:userID', $storage->deleteCalls[0]['key']);
        $this->assertEquals('test_experiment', $storage->deleteCalls[0]['config_name']);
    }

    public function testPersistentStorageWithStatsigOptions()
    {
        $storage = new MockPersistentStorage();

        $options = new StatsigOptions(
            persistent_storage: $storage
        );

        $this->assertNotNull($options->__ref);
        $this->assertNotEquals(0, $options->__ref);
    }
}
