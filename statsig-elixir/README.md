# Statsigelixir
Statsig Elixir Core is a project that utilize functional programing, calls into pre-built binary files written in rust for all operations.

More documentation can be found [doc](https://docs.statsig.com/server-core/elixir-core)
## Release
1. First build binary -- specifically statsig-elixir rust project. 
   a. Utilize build action to build for different targets. After build action being run, libstatsig_elixir-{version}-{nif_version}-{target}.so files will be uploaded 
2. Have release ready in public repo, and attach all compressed libstatsig_elixir-{version}-{nif_version}-{target}.so.tar.gz should be uploaded to release
3. Bump version
4. Generate checksum!! Run FORCE_STATSIG_NATIVE_BUILD="true" mix rustler_precompiled.download NativeBindings --all --print which will include a checksum file
5. run mix hex.publish

## Installation
```elixir
def deps do
  [
    {:statsig_elixir, "~> 0.0.6-beta.7"}
  ]
end
```
