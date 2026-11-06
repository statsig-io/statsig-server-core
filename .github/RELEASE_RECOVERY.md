# SDK release failure audit and recovery plan

## Verified release state

The 0.24.1 public tag names `a3078386c9ed83eb2e8a3d701138efeed680bcf8`.
The GitHub Release exists and Rust 0.24.1 has published. At audit time, npm's
`@statsig/statsig-node-core/0.24.1` and PyPI's `statsig-python-core/0.24.1`
endpoints returned 404. Version 0.24.0 exists on both registries, and all nine
non-Rust publish jobs in its release run completed successfully.

Overall workflow conclusions are not publication receipts. Inspect each
publishing job and its registry entry independently before retrying anything.

## Findings and implemented repairs

| Finding | Publication impact | Repair in this stack |
| --- | --- | --- |
| Seven macOS ARM64 builds were cancelled with `The configuration 'macos-14-arm64-us-default' is not supported` | Direct blocker: `build / publish` depends on the complete build matrix | Move the ARM64 entries in both build matrix files to supported `macos-15`; preserve the architecture and target triple |
| Three RC-check workflows have a Node setup step without `uses` | Independent invalid workflows; they are not dependencies of the SDK publisher | Restore SHA-pinned `actions/setup-node` instructions, retaining the existing runtime input and private-repository guards |
| Node test attempts timed out while flushing events; the second attempt logged zstd decompression failure and an undefined recorded body | Validation failure, independent of the publish dependency | Give each mock request a unique temporary directory, parse ordinary JSON before recording, reject invalid zstd, and let the OS assign mock ports. Add regressions that force identical timestamps across concurrent requests |
| Normal PR builds omit macOS ARM64 | Leaves a gap between PR validation and the release matrix | Add a manual `full_build` preflight using the same build planner. It does not publish because publication still requires a `release` event |
| Malformed workflow files were not rejected by a dedicated CI check | Lets configuration defects survive into releases | Validate every active workflow with checksum-verified actionlint; test build-plan selection with standard-library unit tests |
| LRS Python image job exceeded its ten-minute deadline after the image was already pushed | LRS deployment status, not package publication; retrying may repeat an external write | Allow twenty minutes for cold image builds, upload, and cleanup; confirm the existing image digest before any LRS retry |

The zstd regressions fail against the original helper. Their passing after the
repair proves the mock defects are fixed; a complete Node bindings run is still
required before concluding every observed timeout is resolved.

## Separate repository follow-up

The LRS deployment has a separate private consumer compile issue. Track its
diagnosis and repair in the private LRS follow-up, and validate the consumer
against the intended SDK source before retrying deployment. This consumer
error does not block the package-publishing job.

## Validation and rollout

1. Review the private PR stack from the macOS runner fix upward and run workflow validation, build-plan tests,
   the focused HTTP regressions, and the full Node bindings suite.
2. After the branch is pushed, manually dispatch **Main** on the exact repair
   branch with `full_build: true`. Check every available native target,
   particularly all seven macOS ARM64 package builds. Confirm the resulting
   binaries retain the intended architecture and minimum supported macOS.
   Existing private-repository Windows ARM64 exclusions still apply; this is
   not proof of the public-only Windows ARM64 lane.
3. Merge the reviewed fixes into private main and synchronize them into `rc`
   using the normal ancestry-preserving release flow. Validate the exact stable
   candidate again. The current main line is already at a 0.24.2 beta; recommend
   a normal 0.24.2 release after approval, with matching SDK metadata and C++
   suffix handling.
4. Keep the published 0.24.1 tag and Rust package intact. Rerunning its original
   workflow will still use the old runner selection. If 0.24.1 recovery is
   required instead, first review a dedicated recovery workflow that builds its
   exact immutable source with corrected CI and only publishes missing
   artifacts; do not blindly rerun Rust or recreate the release/tag.
5. After release authorization, observe each publisher and independently verify
   npm, PyPI, Maven Central, NuGet, Hex, crates.io, Go module tags/proxy, and the
   PHP/C++/FFI release assets. Exercise disposable consumers and record the
   source SHA, version and outcome per SDK.

No package-publishing credentials or post-build publishing behavior have been
validated by the local checks. The release's `publish` job currently depends
on the build matrix, not the sibling tests job; require passing candidate tests
before authorizing publication.

## Evidence

- [0.24.1 release run, first attempt](https://github.com/statsig-io/statsig-server-core/actions/runs/42141472402/attempts/1)
- [Cancelled macOS ARM64 build and annotation](https://github.com/statsig-io/statsig-server-core/actions/runs/42141472402/job/127643378529)
- [Node bindings failure, second attempt](https://github.com/statsig-io/statsig-server-core/actions/runs/42141472402/job/127665199308)
- [Successful Rust publication](https://github.com/statsig-io/statsig-server-core/actions/runs/42141472320)
- [Successful 0.24.0 Node publication](https://github.com/statsig-io/statsig-server-core/actions/runs/40718603088/job/123042622006)
- [Independent LRS deployment run](https://github.com/statsig-io/statsig-server-core/actions/runs/42141474495)
- [GitHub macOS 14 retirement](https://github.com/actions/runner-images/issues/13518)
