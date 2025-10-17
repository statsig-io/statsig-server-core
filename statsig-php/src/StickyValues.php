<?php

namespace Statsig;

class StickyValues
{
    public bool $value;
    public $json_value;  // Any type, can be null, array, or object
    public string $rule_id;
    public ?string $group_name;
    public array $secondary_exposures;
    public array $undelegated_secondary_exposures;
    public ?string $config_delegate;
    public ?array $explicit_parameters;
    public int $time;
    public ?int $config_version;

    public function __construct(
        bool $value,
        $json_value,
        string $rule_id,
        ?string $group_name,
        array $secondary_exposures,
        array $undelegated_secondary_exposures,
        ?string $config_delegate,
        ?array $explicit_parameters,
        int $time,
        ?int $config_version = null
    ) {
        $this->value = $value;
        $this->json_value = $json_value;
        $this->rule_id = $rule_id;
        $this->group_name = $group_name;
        $this->secondary_exposures = $secondary_exposures;
        $this->undelegated_secondary_exposures = $undelegated_secondary_exposures;
        $this->config_delegate = $config_delegate;
        $this->explicit_parameters = $explicit_parameters;
        $this->time = $time;
        $this->config_version = $config_version;
    }

    public static function fromArray(array $data): self
    {
        return new self(
            $data['value'] ?? false,
            $data['json_value'] ?? null,
            $data['rule_id'] ?? '',
            $data['group_name'] ?? null,
            $data['secondary_exposures'] ?? [],
            $data['undelegated_secondary_exposures'] ?? [],
            $data['config_delegate'] ?? null,
            $data['explicit_parameters'] ?? null,
            $data['time'] ?? 0,
            $data['config_version'] ?? null
        );
    }

    public function toArray(): array
    {
        $result = [
            'value' => $this->value,
            'json_value' => $this->json_value,
            'rule_id' => $this->rule_id,
            'group_name' => $this->group_name,
            'secondary_exposures' => $this->secondary_exposures,
            'undelegated_secondary_exposures' => $this->undelegated_secondary_exposures,
            'config_delegate' => $this->config_delegate,
            'explicit_parameters' => $this->explicit_parameters,
            'time' => $this->time,
        ];

        if ($this->config_version !== null) {
            $result['config_version'] = $this->config_version;
        }

        return $result;
    }
}
