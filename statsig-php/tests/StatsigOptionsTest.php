<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigOptions;

class StatsigOptionsTest extends TestCase
{
    public function testCreateAndRelease()
    {
        $options = new StatsigOptions();
        $this->assertNotNull($options->__ref);

        $options->__destruct();

        $this->assertNull($options->__ref);
    }
}
