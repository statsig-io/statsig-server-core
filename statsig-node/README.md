<h1 align="center">
	<a href="https://statsig.com/?ref=gh_server_core">
		<img src="https://github.com/statsig-io/js-client-monorepo/assets/95646168/ae5499ed-20ff-4584-bf21-8857f800d485" />
	</a>
    <div />
	<a href="https://statsig.com/?ref=gh_server_core">Statsig</a>
</h1>

<p align="center">
	<a href="https://github.com/statsig-io/statsig-server-core/blob/main/LICENSE">
        	<img src="https://img.shields.io/badge/license-ISC-blue.svg?colorA=1b2528&colorB=ccfbc7&style=for-the-badge">
    	</a>
	<a href="https://www.npmjs.com/package/@statsig/statsig-node-core">
        	<img src="https://img.shields.io/npm/v/@statsig/statsig-node-core.svg?colorA=1b2528&colorB=b2d3ff&style=for-the-badge">
    	</a>
	<a href="https://statsig.com/community?ref=gh_server_core">
        	<img src="https://img.shields.io/badge/slack-statsig-brightgreen.svg?logo=slack&colorA=1b2528&colorB=FFF8BA&style=for-the-badge">
    	</a>
</p>

Statsig helps you move faster with feature gates (feature flags), and/or dynamic configs. It also allows you to run A/B/n tests to validate your new features and understand their impact on your KPIs. If you're new to Statsig, check out our product and create an account at [statsig.com](https://www.statsig.com/?ref=gh_server_core).


## Getting Started

Read through the Node [Documentation](https://docs.statsig.com/server-core/node-core?ref=gh_server_core) or check out the [Samples](https://github.com/statsig-io/statsig-server-core/tree/main/examples/node).

## Amplitude Analytics

The Rust core implements the Amplitude-Java client, with a Node binding exported
from the same package. No other language bindings are added.

```typescript
import { Amplitude } from '@statsig/statsig-node-core';

const amplitude = Amplitude.getInstance();
amplitude.init(process.env.AMPLITUDE_API_KEY!);
amplitude.setCallbacks((event, status, message) => {
  console.log(event.eventType, status, message);
});
await amplitude.logEvent({ eventType: 'Song Played', userId: 'user-123' });
await amplitude.flushEvents();
await amplitude.shutdown();
```

The client supports named instances, Java event fields, batching, middleware,
callbacks, plans/ingestion metadata, custom headers and proxies, throttling hints,
per-user/device retries, separate send/retry concurrency, and shutdown. The original
`new Amplitude()`, `init(key, {serverUrl, timeoutMs})`, and immediate `track(event)`
remain supported. See [the compatibility guide](../statsig-rust/AMPLITUDE.md) for
all Java API mappings, retry behavior, and Rust/Node lifecycle adaptations.

### CLI kitchen sink

After building the Node addon, run from the repository root:

```bash
node statsig-node/scripts/amplitude-kitchen-sink.cjs
# Print SDK logs and individual callbacks:
node statsig-node/scripts/amplitude-kitchen-sink.cjs --verbose
# Or use the package script:
npm --prefix statsig-node run amplitude:kitchen-sink
```

The script uses the Amplitude API key hardcoded in its source and sends nine
clearly marked demo events to the live project, with a unique user and run ID.
It exercises named and independent clients, configuration, rich event/revenue and
group fields, plans/ingestion metadata, middleware continuations and drops, all
`logEvent` overloads, both callback levels, threshold/period flushing, regular and
Batch APIs, explicit flushing, clearing settings, local validation, and shutdown.
Each scenario checks its result; a failed delivery or timeout gives a nonzero exit
code. `--help` lists usage without sending events. Build with
`./tore build node --no-docker` first if the local addon is missing or out of date.

## Packages

Bindings

- Node [[npm](https://www.npmjs.com/package/@statsig/statsig-node-core)] [[source](https://github.com/statsig-io/statsig-server-core/blob/main/statsig-node)] [[docs](https://docs.statsig.com/server-core/node-core?ref=gh_server_core)] 

- Python [[pypi](https://pypi.org/project/statsig-python-core)] [[source](https://github.com/statsig-io/statsig-server-core/blob/main/statsig-pyo3)] [[docs](https://docs.statsig.com/server-core/python-core?ref=gh_server_core)] 

- PHP [[packagist](https://packagist.org/packages/statsig/statsig-php-core)] [[source](https://github.com/statsig-io/statsig-server-core/blob/main/statsig-php)] [[docs](https://docs.statsig.com/server-core/php-core?ref=gh_server_core)] 

- Java [[maven](https://search.maven.org/artifact/com.statsig/javacore)] [[source](https://github.com/statsig-io/statsig-server-core/tree/main/statsig-java)] [[docs](https://docs.statsig.com/server-core/java-core?ref=gh_server_core)] 

- Rust [[crates.io](https://crates.io/crates/statsig-rust)] [[source](https://github.com/statsig-io/statsig-server-core/tree/main/statsig-rust)] [[docs](https://docs.statsig.com/server-core/rust-core?ref=gh_server_core)] 

- Elixir [[hex](https://hex.pm/packages/statsig_elixir)] [[source](https://github.com/statsig-io/statsig-server-core/tree/main/statsig-elixir)] [[docs](https://docs.statsig.com/server-core/elixir-core?ref=gh_server_core)] 



## Community

If you need any assitance or just have a question, feel free to reach out to us on [Slack](https://statsig.com/community?ref=gh_server_core).
