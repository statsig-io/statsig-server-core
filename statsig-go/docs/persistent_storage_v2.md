# Structured persistent storage in Go

`NewPersistentStorageV2` supports JSON-valued experiment and layer parameters. Existing storage APIs keep their string-only behavior and source signatures. To fix non-string persistence, migrate the storage callbacks, application loader, and both experiment and layer options together.

## Migration example

This in-memory byte store illustrates the serialization boundary. Substitute the application's database operations and error reporting. The callbacks do not return errors; production adapters need their own handling for storage failures. Keep the storage object reachable until the client has shut down, as with the legacy API.

```go
var rows sync.Map // storage key -> serialized UserPersistedValuesV2
var writeMu sync.Mutex // demo only; use database atomic updates in production

load := func(key string) (*statsig.UserPersistedValuesV2, error) {
    raw, ok := rows.Load(key)
    if !ok { return nil, nil }
    var values statsig.UserPersistedValuesV2
    if err := json.Unmarshal(raw.([]byte), &values); err != nil {
        return nil, err
    }
    return &values, nil
}
storage := statsig.NewPersistentStorageV2(statsig.PersistentStorageFunctionsV2{
    Load: func(key string) *statsig.UserPersistedValuesV2 {
        values, err := load(key)
        if err != nil { log.Printf("load assignment: %v", err); return nil }
        return values
    },
    Save: func(key, name string, value statsig.StickyValuesV2) {
        writeMu.Lock()
        defer writeMu.Unlock()
        values, err := load(key)
        if err != nil { log.Printf("read assignment: %v", err); return }
        if values == nil { v := statsig.UserPersistedValuesV2{}; values = &v }
        (*values)[name] = value
        raw, err := json.Marshal(values)
        if err != nil { log.Printf("save assignment: %v", err); return }
        rows.Store(key, raw)
    },
    Delete: func(key, name string) {
        writeMu.Lock()
        defer writeMu.Unlock()
        values, err := load(key)
        if err != nil { log.Printf("read assignment: %v", err); return }
        if values == nil { return }
        delete(*values, name)
        raw, err := json.Marshal(values)
        if err != nil { log.Printf("delete assignment: %v", err); return }
        rows.Store(key, raw)
    },
})
// Install storage with the existing NewOptionsBuilder().WithPersistentStorage(storage).
// Use the same storage key convention as your adapter, e.g. userID + ":userID".
values, err := load(storageKey)
if err != nil { return err } // stop; do not evaluate with an accidental empty map
if values == nil { empty := statsig.UserPersistedValuesV2{}; values = &empty }
experiment := client.GetExperimentWithOptionsV2(user, "my_experiment",
    &statsig.ExperimentEvaluationOptionsV2{UserPersistedValues: values})
layer := client.GetLayerWithOptionsV2(user, "my_layer",
    &statsig.LayerEvaluationOptionsV2{UserPersistedValues: values})
```

A nil persisted-map pointer opts out. A nonnil pointer to an empty map opts in with no previous assignments. Existing enforcement and exposure flags are available on both V2 option types. Opt-out evaluations can issue Delete, following existing native behavior. The application loads records and supplies them to evaluation; an internal native test helper is not a production loading API.

Stored records require an explicit boolean `value`, including `false`. If any entry is missing this field or supplies null, discard the partial decode and propagate the error before calling experiment or layer evaluation.

Do not treat a failed database decode as an empty record in production: choose an explicit error/retry policy and preserve existing data. The example is intentionally a small demonstration; production merge/write/delete operations must be atomic across processes, preserve TTL, and avoid overwriting assignments on read failures.

## Values and numbers

Decoded parameters are `string`, `bool`, `map[string]any`, `[]any`, `nil`, and `json.Number`. Decode database bytes directly into `UserPersistedValuesV2`; decoding into an intermediate generic map can round large integers before V2 sees them. Inspect a number with `json.Number.Int64` or `Float64`, checking the returned error; use `strconv.ParseUint(string(n), 10, 64)` for a native-supported unsigned integer above int64 maximum.

Supported integer tokens span signed int64 through unsigned uint64. Out-of-range integer tokens are rejected rather than silently rounded. Decimal/exponent tokens must fit a finite float64 in native code. Go serialization that produces an out-of-range integer token is also rejected, even if the original Go value was a float. Invalid `json.Number`, NaN/infinity, byte slices, non-string map keys, unsupported Go values, and cyclic/excessively deep containers are rejected before Load or evaluation transport. Parameters should be JSON primitives and containers, not custom Go structs or pointers. Missing/null top-level `json_value` is the native optional value; null inside the parameter map remains a present nil entry, distinguishable from a missing key.

V2 avoids additional Go persistence rounding of native-supported integers. Native evaluation can already normalize exponent spelling and round decimal values. Existing experiment/layer getters retain their float64 behavior and are not exact-integer accessors. Known assignment metadata is retained; unknown record fields are ignored, so V2 is not an opaque forwarder of future metadata. Missing, null, or programmatically nil `SecondaryExposures` serialize as an empty array because the native field requires an array. Marshaling does not change the caller's record; optional `UndelegatedSecondaryExposures` keeps its existing nullable behavior. Time must be nonnegative and config version must fit native u32.

## Shared storage and rollback

Legacy string-only records can be read through V2. Legacy Go readers can reject a newly written non-string record, potentially invalidating a whole user map. Source compatibility of the additive API does not change that data compatibility issue.

Upgrade all readers, writers, jobs and rollback binaries sharing a namespace before enabling new typed writes. Otherwise use an explicit namespace migration that preserves existing assignments, metadata and TTL and accounts for concurrent writers and deletion. Isolation alone does not preserve assignment continuity; dual reads can resurrect stale records and dual writes are not automatically safe.

After typed writes begin, retain a reader capable of decoding them for rollback. An old SDK or old namespace snapshot may lose newer assignments. Missed historical saves cannot be reconstructed by this fix. API/release review, supported-platform package validation and customer-specific datastore rollout are still required before release.
