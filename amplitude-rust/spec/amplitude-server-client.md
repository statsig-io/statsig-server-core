# Amplitude Client class

This document describes the implementation of an Amplitude analytics client. The general structure of this is that the Client is a "class" that is instantiated with an api_key.

Types are defined in the [TypeSpec models](amplitude-server-client.tsp).

## constructor

| Input | Type | Required | Behavior |
| --- | --- | --- | --- |
| `api_key` | `string` | Yes | The api key of amplitude project. Must be set properly before tracking events |
| `configuration` | `Config` | No | Configuration fields for the client.  |

## methods

### `track`

Submits one analytics event.

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `event` | `Event` | Yes | The event to submit. |
| `callback` | `function(event: Event, statusCode: int, message?: string)` | No | Reports success or failure with the original event, status code, and optional error message. Client implementations should not necessarily implement this parameter, but rather use whichever asynchronous pattern is appropriate for the language it's developed in. |

### `identify`

Uses `track` to submit an event with `event_type` set to `$identify`.

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `operations` | `Identify` | Yes | Operations used to populate `user_properties` on the event. |
| `options` | `EventOptions` | Yes | Supplies the user's identity and other event options. |
| `event_properties` | `JsonObject` | No | Additional properties attached to the event. |
| `callback` | `function(event: Event, statusCode: int, message?: string)` | No | Reports success or failure with the generated `$identify` event, status code, and optional error message. Same as "track.callback". |

