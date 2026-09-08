# Project agent memory

This file is the project's committed home for project-intrinsic agent knowledge: build, test, release, architecture, and sharp-edge notes that should travel with the code.

- Add durable project-specific notes here as they are discovered through real work.

## Running the Go tests

`statsig-go` loads the native library from published `go-server-core-binaries-*`
modules pinned in `statsig-go/go.mod`. That pin lags this repo, so tests panic in
`purego.RegisterLibFunc` with `symbol not found` when the Go bindings reference an
FFI function the pinned binary predates. Build the library from this checkout and
point the loader at it with the `STATSIG_LIB_PATH` override in
`statsig-go/statsig_ffi.go`:

```
cargo build -p statsig_ffi --release
cd statsig-go && STATSIG_LIB_PATH=$(git rev-parse --show-toplevel)/target/release/libstatsig_ffi.dylib go test ./...
```

Use `.so` instead of `.dylib` on Linux.

## Maintaining this file

Keep this file for knowledge useful to almost every future agent session in this project.
Do not repeat what the codebase already shows; point to the authoritative file or command instead.
Prefer rewriting or pruning existing entries over appending new ones.
When updating this file, preserve this bar for all agents and keep entries concise.
