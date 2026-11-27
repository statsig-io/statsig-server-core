# Amplitude plugin contract

Plugins process and deliver events through a [Timeline](amplitude-timeline.md).
The [plugin types](amplitude-plugins.tsp) define `PluginType`. The
[server client types](amplitude-server-client.tsp) define `Event` and `Result`.

## BasePlugin

### constructor

create plugin with provided `PluginType`

| Input | Type | Required | Behavior |
| --- | --- | --- | --- |
| `plugin_type` | `PluginType` | Yes | the type of plugin that this is |

### methods

#### `setup`

This method gets invoked either when client is initialized, or after the plugin is added to an existing client

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `client` | `Client` | Yes | Supplies the client and its configuration to the plugin. |

#### `execute`

This method gets invoked when apply is called on timeline. Returns the mutated version of the `Event` or returns `None` in which case that event will be filtered out from being flushed by the destination plugin

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `event` | `Event` | Yes | The current event being processed by this plugin. |

### properties

| Property | Type | Behavior |
| --- | --- | --- |
| `plugin_type` | `PluginType` | Holds the stage assigned at construction. |

## Before Plugin

Extends `BasePlugin` and runs before enrichment. `PluginType` is 'before'

## Enrichment Plugin

Extends `BasePlugin` and runs after the before stage. `PluginType` is 'enrichment'


## DestinationPlugin

Extends `BasePlugin` and delivers events after enrichment. `PluginType` is `destination`

### constructor

Sets `plugin_type` to `destination`. No parameters.

### methods

#### `execute`

Submits the event to the destination and returns a `Result`. The timeline
invokes every destination and returns the first destination's result.

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `event` | `Event` | Yes | The event that completed the before and enrichment stages. |

| Result field | Type | Behavior |
| --- | --- | --- |
| `event` | `Event` | The event associated with the delivery result. |
| `code` | `float64` | The destination's outcome code. |
| `message` | `string` | The destination's outcome message. |

#### `flush`

Requests delivery of pending events. The timeline collects the value returned
by each destination. No parameters.

#### `shutdown`

Stops the destination and releases its resources. The timeline calls this on
destinations in registration order. No parameters.

## AmplitudeDestinationPlugin

Extends `DestinationPlugin` and sends events to Amplitude. The client registers
an instance of this by default.

### constructor

Keeps a reference to "client.configuration" again. Creates an in-memory storage that can be used to hold a queue of events.

### methods

#### `setup`

Extends from DestinationPlugin like usual

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `client` | `Client` | Yes | Supplies the configuration and its configured storage provider. |

#### `execute`

Pushes the event to the queue. Schedules the queue to `flush` later, depending on the logic provided by the `client`. e.g.) if `flushIntervalMillis=2000` schedule it to flush in 2 seconds, if it isn't already scheduled


| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `event` | `Event` | Yes | The event received from the client's timeline. |

| Result | Type | Behavior |
| --- | --- | --- |
| Delivery outcome | `Result` | Completes when the workers report the delivery outcome for the queued event. The result carries the processed event, code, and message. |

#### `flush`

Takes the queue of stored events, breaks them up depending on client provided `flush_queue_size` and then sends all the events to the client provided `serverUrl` via HTTP.

#### `shutdown`

`flush` all unsent queued events, regardless of scheduling. Shut down 
