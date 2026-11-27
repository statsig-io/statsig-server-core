# Amplitude Timeline class


## constructor

Creates empty plugin lists that holds a list of Plugin instances

| Input | Type | Required | Behavior |
| --- | --- | --- | --- |
| `configuration` | `Config` | No | Stores the supplied configuration. Provide one here or through `setup` before processing an event. |

## methods

### `setup`

Invoked in the constructor of the client. Takes client as argument and holds a reference to the client's "configuration" property

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `client` | `Client` | Yes | Supplies the configuration used for processing and logging. |

### `add`

Appends a plugin to the "plugins" list

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `plugin` | `Plugin` | Yes | instance of the Plugin interface. |

### `remove`

Removes a plugin.

| Parameter | Type | Required | Behavior |
| --- | --- | --- | --- |
| `plugin` | `Plugin` | Yes | removes this specific plugin from the plugins list. |

### `flush`

Every plugin that is of type `Destination` in the plugins list has it's flush called on it. Whatever these flush calls return, return as a list.

No parameters

### `apply`

Returns [Result]

Run all of the `execute` methods on every `Before` plugin. Then run any `execute` method on every `Enrichment` plugin. If any of the plugins return `None` than that means the event is filtered out and don't proceed any further. Return result with "code=0" .

Next, run `execute` on every `Destination` plugin. Return the `Result` of the first invokation of the plugin. If there was no `Destination` plugins, then "code=100" and message is `Event not tracked, no destination plugins on the instance`

### `shutdown`

Calls `shutdown` on destinations in registration order

## properties

| Property | Type | Behavior |
| --- | --- | --- |
| `logger` | `Logger` | uses reference to provided clients' logger, if available. |
