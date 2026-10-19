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

## Migrating ID-list DataStore snapshots

Core defaults ID lists off, unlike Legacy Node. Set `enableIdLists: true` and
have the existing DataStore explicitly return `true` from
`supportsFeature('id_list_snapshot')` to enable snapshot reads. Config-spec
support alone does not opt an adapter into ID-list keys.

Initialization reads the manifest and every referenced child before installing
the complete candidate. `supportPollingUpdatesFor('id_list_snapshot')` returning
`true`, or having no preference, reloads that cache on `idListsSyncIntervalMs`.
An explicit `false` still allows cache-first initialization but assigns later
refreshes to HTTP. Missing values, malformed snapshots, or rejected reads cause
whole-refresh HTTP fallback; no partial cache or HTTP candidate is installed.
If both sources fail, refresh retains the previous complete snapshot, and an
initialization failure is observable through the ordinary initialization result.

Legacy-compatible children contain condensed `+hash` entries. The manifest's
`size` is the original remote change-log byte cursor, **not** the condensed child
length or member count. A parseable Legacy cache may still be stale relative to
that cursor after a failed Legacy download. The first HTTP reconciliation after
accepting a cache snapshot therefore downloads complete lists from byte zero and
replaces membership atomically. Only a successful reconciliation establishes
cursors for later incremental HTTP requests. Failed reconciliation leaves the
previous snapshot intact and retries from zero.

The fixed-key manifest/children format is not transactional: reading every child
and atomically installing the in-memory candidate cannot prove that an external
writer published all keys in one generation. Fail-closed/no-Statsig-fallback mode
is deferred; this phase intentionally retains HTTP fallback. Do not treat config
specs fallback settings as an ID-list no-network guarantee. ID-list write-back is
a separate follow-up to this read-through change.

### Fork limitation

Automatic post-fork refresh of DataStore-backed ID lists is not supported in this
change. Fork before constructing and initializing the SDK and DataStore in each
worker; recovery of an already-initialized, inherited DataStore-backed instance
is deferred. Existing best-effort restart logic is not a fork-safety guarantee.
Ordinary non-fork reads, polling, HTTP fallback, and shutdown remain in scope.

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
