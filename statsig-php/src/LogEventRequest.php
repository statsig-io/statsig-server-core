<?php

namespace Statsig;

class LogEventRequest
{
    public int $event_count;
    public int $retries;
    public LogEventPayload $payload;

    public function __construct(string $raw_result)
    {
        $result = json_decode($raw_result, true);
        if ($result === null) {
            $this->event_count = 0;
            $this->retries = 0;
            $this->payload = new LogEventPayload([], []);
            return;
        }

        $this->event_count = $result['eventCount'];
        $this->retries = $result['retries'];

        $payload = $result['payload'];
        $this->payload = new LogEventPayload($payload['events'], $payload['statsigMetadata']);
    }
}
