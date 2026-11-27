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

Reuse the client across calls. Each `track` or `identify` call waits for the
ingestion response and returns `nil` on success. Invalid input, network
failures, and rejected requests raise `Amplitude::Error`.
Pass a callable as `callback:` or supply a block to receive the original
event hash, HTTP status code, and optional error message before the call
returns or raises. A status code of zero means there was no HTTP response.

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

Run the local HTTP integration tests from the repository root.

```sh
ruby -I amplitude-ruby/lib amplitude-ruby/test/client_test.rb
```
