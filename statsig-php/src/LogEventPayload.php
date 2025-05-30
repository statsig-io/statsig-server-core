<?php

namespace Statsig;

/**
 * LogEventPayload can be sent as a POST request to Statsig via the HTTP API.
 * See https://docs.statsig.com/http-api/#log-an-event for more details.
 *
 * Body: { "events": [...], "statsigMetadata": {...} }
 *
 * @property array $events
 * @property array $statsig_metadata
 */
class LogEventPayload
{
    public array $events;
    public array $statsig_metadata;

    public function __construct(array $events, array $statsig_metadata)
    {
        $this->events = $events;
        $this->statsig_metadata = $statsig_metadata;
    }
}
