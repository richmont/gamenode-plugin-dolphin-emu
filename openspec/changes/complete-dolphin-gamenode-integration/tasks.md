# Tasks

Implementation is pending. This checklist owns the MVP execution detail; root TASKS.md retains
the broader roadmap. Keep both synchronized and record evidence before checking any item.

## 1. Host contract prerequisites

- [ ] 1.1 Record the SDK/host revision, required permissions and independent revision domains in protocol notes; verify against the public API and a selected-game fixture.
- [x] 1.2 Demonstrate host-approved Dolphin executable acquisition and runtime ownership with an integration fixture; document any missing host UI/API as a blocking GameNode dependency before implementing launch. Evidence: `launch/tests/launch_reference_host.rs` exercises `RuntimesRequestAccess` with no executable handle, the `dolphin-emu` / `dolphin-emu.desktop` hints, a handle-free reply, durable reuse and reapproval on a changed revision. The real picker UI is desktop-side and remains part of native acceptance 5.3. Documented gaps: `LibraryEntry` has no provider `rom`/`platform`/`title` fields and content approvals have no digest (`docs/launch-session-evidence.md`).
- [x] 1.3 Verify first-use proposal/review/execution and repeat-action intent semantics in a reference-host fixture; document when the intent is consumed and how approval state is recovered. Evidence: the fixture asserts first-use proposal plus request, exact intent forwarding, repeated Play reuse without a second proposal, stale-recipe repair only before a job exists, and no retry after transport errors. Host intent consumption/confirmation is host code (`request_action`); the fixture verifies the plugin side and the host contract is documented in `docs/protocol-notes.md`.

## 2. Identity and local metadata

- [ ] 2.1 Add explicit disc metadata with legacy field readers; verify catalog IDs never come from disc fields and document the field transition.
- [ ] 2.2 Implement copy/revision/multidisc identity and deduplication with host reconciliation fixtures preserving existing matches/history; document the migration strategy before changing record IDs.
- [ ] 2.3 Locate local Dolphin title files and implement bounded parsing and precedence; test user/system/English fallbacks, malformed or missing files and readable ROM/file-name fallbacks, then document supported data layouts.
- [ ] 2.4 Add explicit scan depth/entry-limit, symlink and overlapping-root regressions while retaining worker offloading; document remaining compressed-format limitations.

## 3. Persistence and catalog association

- [x] 3.1 Publish the minimal durable disc catalog using SDK data_publish, optional data.publish/storage.persist grants and stable producer-scoped resource IDs; verify grant denial, replacement, payload privacy and reference-host restart, and document evidence (`docs/durable-catalog-evidence.md`).

- [ ] 3.1a Add versioned provider cache records through persistent Storage and matching capabilities/manifests; verify granted/denied storage, restart, schema migration, cleanup, quota and revision-conflict fixtures.
- [ ] 3.2 Implement library-first matching and candidate resolution through the SDK; verify user choices survive, ambiguities stay unresolved and a private cache entry never authorizes launch, then document the matching flow.
- [ ] 3.3 Implement extension bookkeeping in its own Storage namespace; verify component/account isolation and absence of session handles, intents and credentials in stored records, and document recovery behavior.

## 4. Approved launch session

- [x] 4.1 Capture host action context and intent and resolve the matched library entry/ROM handle; test missing target, unmatched entry, denied file access, missing ROM and readiness-only sessions without execution. Evidence: `launch/src/lib.rs` rejects a missing/empty target and an unresolved catalog match; `launch/tests/launch_reference_host.rs` covers unmatched entry, denied files, missing ROM, no intent and `Initialize`/`Check` without any resource call or job. A real missing target is also rejected by `SdkHost::new`.
- [x] 4.2 Resolve/register the approved owned runtime and stable content binding; add extension discovery permission, verify exact grants and independent binding/library revisions, and update permission documentation. Evidence: runtime reuse/request, content revision 0 only for creation, separate recipe revision, manifest additions (`game.launch`, `library.discovery`, `library.read.all`, storage/notification grants) and `docs/protocol-notes.md`.
- [x] 4.3 Propose and request immutable recipes using host consent and current intents; verify exact RPC sequence and no execution overrides or plugin process spawn, including refusals and expiry. Evidence: the fixture asserts no executable/argv/cwd/environment/URI/mod-layer payload; denied launch and no-intent paths create no job; transport errors are not retried. Intent expiry itself is host-side and is exercised in native acceptance 5.3.
- [x] 4.4 Reconcile durable IDs on restart/repeated Play; test no duplicate bindings/recipes and reapproval on changed runtime/content/grants, and document stale-state troubleshooting. Evidence: repeated Play reuses runtime/content/recipe without re-registering or re-proposing; changed runtime revision re-enters the picker; changed ROM name/size/mtime and changed content revision force a new binding and recipe; storage records contain no handles or intents. `docs/architecture.md` and `docs/launch-session-evidence.md` document recovery.

## 5. MVP packaging and acceptance

- [x] 5.1 Build and verify both signed Linux x86_64 development packages with non-placeholder keys kept outside the repo; verify binary/manifest targets and permission parity, and document reproducible packaging commands and dependency notices (`docs/durable-catalog-evidence.md`). Both manifests declare `supported_platforms` and a signed `impact_if_disabled` per permission, re-signed with the same development key. **Superseded for the launch component (2026-11-22):** the launch manifest and binary changed; rebuild, re-sign and re-verify before native acceptance without reusing the recorded hashes.
- [ ] 5.2 Add CI for formatting, Clippy, workspace tests and package verification; verify the pipeline uses an external Cargo target and document SDK/API compatibility policy.
- [ ] 5.3 Run native GameCube and Wii discovery/matching/launch/repeated-Play acceptance in GameNode; record consent, cancellation, missing ROM, exit detection and playtime evidence with host/plugin revisions. **Open.** The in-repo gRPC reference host (`docs/launch-session-evidence.md`) proves only the client contract, not the real picker UI, confirmation dialog, supervisor or playtime path.
- [x] 5.4 Run formatting, Clippy, workspace tests, git diff --check and strict OpenSpec validation; synchronize README, architecture, protocol and root backlog with actual completed behavior.

- [x] 5.5 Bundle the signed Linux provider and extension under dev.richmont.gamenode.dolphin-emu.bundle 0.1.0; verify signatures, common publisher, exact two components and SHA-256 evidence (`docs/durable-catalog-evidence.md`). Native host acceptance remains in 5.3 and main-repo integration. **Superseded for the launch component (2026-11-22):** rebuild the bundle after the launch change; the existing artifacts and hashes are historical.

## Workflow follow-up

- Keep Windows releases, possible macOS/additional architectures, covers, saves and bundles in the root roadmap; each future release adds real binaries, manifest targets and native evidence.
- Archive this change only after its implementation and acceptance evidence exist; do not archive planning completion.
