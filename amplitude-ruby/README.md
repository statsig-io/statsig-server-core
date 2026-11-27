# Amplitude Ruby

Track events and update user properties using Ruby's standard library. The
[server client contract](../amplitude-rust/spec/amplitude-server-client.md)
describes `track` and `identify` and links to their TypeSpec data models.

From the repository root, add `amplitude-ruby/lib` to Ruby's load path.

```sh
ruby -I amplitude-ruby/lib your_script.rb
irb -I amplitude-ruby/lib -r amplitude
```

```ruby
require 'amplitude'

amplitude = Amplitude.new(ENV.fetch('AMPLITUDE_API_KEY'))
operations = Amplitude::Identify.new.set('plan', 'pro').add('logins', 1)
amplitude.identify(
  operations,
  user_id: 'user-123',
  event_properties: { source: 'account' }
)
amplitude.track('Purchase Completed', user_id: 'user-123', event_properties: { total: 29.99 })
```

Reuse the client across calls. With the default flush interval of zero,
`track` and `identify` wait for the ingestion response and return `nil` on
success. Invalid input and delivery failures raise `Amplitude::Error`.

Set `flush_interval_millis` to a positive value to queue events and return
`nil` immediately after enqueueing. A worker sends the batch when the
interval elapses or the queue reaches `flush_queue_size`. Immediate
validation errors still raise on the calling thread.

Pass a callable as `callback:` or supply a block to receive the original
event hash, status code, and optional error message. For queued delivery,
the callback runs on a background thread after delivery, including failures.
A status code of zero means there was no HTTP response.

Supply a `user_id` or `device_id`. Pass the event name and keyword arguments to
`track`, or pass an event hash with `event_type` and optional `options`.
For `identify`, pass event options as a hash or as keywords.
Use one identify operation per property and send `clear_all` on its own.
The client fills in a missing timestamp and insert ID. Set an explicit `insert_id`
and reuse it if you retry an operation.

`Amplitude.new` accepts configuration as a hash or as keywords. For example,
pass `server_zone: :eu` for an EU project, `server_url:` for a custom endpoint,
or `timeout_ms:` to override the default 10-second timeout. Use
`flush_interval_millis:` and `flush_queue_size:` to batch concurrent
calls, and `flush_max_retries:` to retry transient failures.
Set `log_level:` to a value from 0 (off) through 4 (debug), and use
`instance_name:` to label the logs.

The client registers an `AmplitudeDestinationPlugin` by default. Register a
custom plugin with `add(plugin)` and unregister it with `remove(plugin)`.
Subclass `Amplitude::BeforePlugin` or `Amplitude::EnrichmentPlugin` and implement
`execute(event)` to return an event or `nil` to filter it out. Custom
destinations subclass `Amplitude::DestinationPlugin` and return an
`Amplitude::Result` or a completion object that exposes the result through
`value`. The timeline invokes all destinations and uses the first result for
the caller's completion.

Call `flush` to send queued events immediately. It returns a list of results
from the registered destinations. Call `shutdown` to drain their queues and
close the client. See the [plugin contract](../amplitude-rust/spec/amplitude-plugins.md)
and [Timeline contract](../amplitude-rust/spec/amplitude-timeline.md).

Run the local HTTP integration tests from the repository root.

```sh
ruby -I amplitude-ruby/lib amplitude-ruby/test/client_test.rb
```

The live smoke tests are for manual runs. They use `AMPLITUDE_API_KEY` from your
shell. One case sends an identify event and three `SDK Smoke Test` events after
a three-second interval. The shutdown case queues an `SDK Smoke Test Shutdown`
event with a ten-second interval and confirms that shutdown sends it before
the timer fires.
Set `AMPLITUDE_SERVER_ZONE=eu` for an EU project. Find the test events using
the printed user ID or the `smoke_run_id` event property.

```sh
ruby -I amplitude-ruby/lib amplitude-ruby/smoke/live.rb
```
