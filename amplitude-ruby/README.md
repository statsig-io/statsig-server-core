# Amplitude Ruby

Track events from Ruby through the `ffi` gem and the native `amplitude-rust` client.
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
amplitude.track(
  'Purchase Completed',
  user_id: 'user-123',
  event_properties: { total: 29.99, currency: 'USD' }
)
```

Reuse the client to track multiple events. Each `track` call waits for the
ingestion response and returns `nil` on success. Invalid input, network
failures, and rejected events raise `Amplitude::Error`.

Supply a `user_id` or `device_id`. Events can also include `user_properties`,
`time` in milliseconds since the Unix epoch, `insert_id`, and `session_id`.
Rust fills in a missing timestamp and insert ID. Set an explicit `insert_id`
and reuse it if you retry an event.

`Amplitude.new` accepts optional `server_url:` and `timeout_ms:` keywords.
The defaults are `https://api2.amplitude.com/2/httpapi` and 10,000 milliseconds.
Set `AMPLITUDE_LIB_PATH` to the absolute library path when using a release build
or a custom Cargo target directory.

Run the local HTTP integration tests after building the library.

```sh
ruby -I amplitude-ruby/lib amplitude-ruby/test/track_test.rb
```
