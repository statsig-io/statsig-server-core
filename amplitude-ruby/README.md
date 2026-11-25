# Amplitude Ruby

Track events and update user properties from Ruby through the `ffi` gem and the native
`amplitude-rust` client. The [server client contract](../amplitude-rust/spec/amplitude-server-client.wit)
defines the client surface. `track` and `identify` are implemented. The other client methods
raise `NotImplementedError`.
Build the shared library from the repository root and add `amplitude-ruby/lib` to
Ruby's load path.

```sh
gem install ffi
cargo build -p amplitude_ruby_ffi --locked
ruby -I amplitude-ruby/lib your_script.rb
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

Supply a `user_id` or `device_id`. Pass the event name and keyword arguments to
`track`, or pass an event hash with `event_type` and optional `options`.
For `identify`, pass event options as a hash or as keywords.
Use one identify operation per property and send `clear_all` on its own.
Rust fills in a missing timestamp and insert ID. Set an explicit `insert_id`
and reuse it if you retry an operation.

`Amplitude.new` accepts configuration as a hash or as keywords. For example,
pass `server_zone: :eu` for an EU project, `server_url:` for a custom endpoint,
or `timeout_ms:` to override the default 10-second timeout.
Set `AMPLITUDE_LIB_PATH` to the absolute library path when using a release build
or a custom Cargo target directory.

Run the local HTTP integration tests after building the library.

```sh
ruby -I amplitude-ruby/lib amplitude-ruby/test/client_test.rb
```
