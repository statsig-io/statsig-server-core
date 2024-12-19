<?php

namespace Statsig;

class StatsigUserBuilder
{
    private ?string $user_id = null;
    private ?array $custom_ids = null;
    private ?string $email = null;
    private ?string $ip = null;
    private ?string $user_agent = null;
    private ?string $country = null;
    private ?string $locale = null;
    private ?string $app_version = null;
    private ?array $custom = null;
    private ?array $private_attributes = null;

    public static function withUserID(string $user_id): StatsigUserBuilder
    {
        return new StatsigUserBuilder($user_id, null);
    }

    public static function withCustomIDs(array $custom_ids): StatsigUserBuilder
    {
        return new StatsigUserBuilder(null, $custom_ids);
    }

    private function __construct(?string $user_id, ?array $custom_ids)
    {
        $this->user_id = $user_id;
        $this->custom_ids = $custom_ids;
    }

    public function withEmail(string $email): StatsigUserBuilder
    {
        $this->email = $email;
        return $this;
    }

    public function withIP(string $ip): StatsigUserBuilder
    {
        $this->ip = $ip;
        return $this;
    }

    public function withUserAgent(string $user_agent): StatsigUserBuilder
    {
        $this->user_agent = $user_agent;
        return $this;
    }

    public function withCountry(string $country): StatsigUserBuilder
    {
        $this->country = $country;
        return $this;
    }

    public function withLocale(string $locale): StatsigUserBuilder
    {
        $this->locale = $locale;
        return $this;
    }

    public function withAppVersion(string $app_version): StatsigUserBuilder
    {
        $this->app_version = $app_version;
        return $this;
    }

    public function withCustom(array $custom): StatsigUserBuilder
    {
        $this->custom = $custom;
        return $this;
    }

    public function withPrivateAttributes(array $private_attributes): StatsigUserBuilder
    {
        $this->private_attributes = $private_attributes;
        return $this;
    }

    public function build(): StatsigUser
    {
        return new StatsigUser(
            $this->user_id ?? "",
            $this->custom_ids ?? [],
            $this->email,
            $this->ip,
            $this->user_agent,
            $this->country,
            $this->locale,
            $this->app_version,
            $this->custom,
            $this->private_attributes
        );
    }
}
