# Statsigelixir
Statsig Elixir Core is a project that utilize functional programing, calls into pre-built binary files written in rust for all operations.

## Release
1. First build binary -- specifically statsig-elixir rust project. 
   a. Utilize build action to build for different targets. After build action being run, libstatsig_elixir-{version}-{nif_version}-{target}.so files will be uploaded 
2. Have release ready in public repo, and attach all compressed libstatsig_elixir-{version}-{nif_version}-{target}.so.tar.gz should be uploaded to release
3. Bump version
4. Generate checksum!! Run FORCE_STATSIG_NATIVE_BUILD="true" mix rustler_precompiled.download NativeBindings --all --print which will include a checksum file
5. run mix hex.publish

## Installation

If [available in Hex](https://hex.pm/docs/publish), the package can be installed
by adding `statsigelixir` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:statsigelixir, "~> 0.1.0"}
  ]
end
```

Documentation can be generated with [ExDoc](https://github.com/elixir-lang/ex_doc)
and published on [HexDocs](https://hexdocs.pm). Once published, the docs can
be found at <https://hexdocs.pm/statsigelixir>.

