<?php

namespace Statsig;

class StatsigEventData
{
    public string $name;
    public string|int|float|null $value;
    public ?array $metadata;

    public function __construct(string $name, string|int|float|null $value = null, ?array $metadata = null)
    {
        $this->name = $name;
        $this->value = $value;
        $this->metadata = $metadata;
    }
}
