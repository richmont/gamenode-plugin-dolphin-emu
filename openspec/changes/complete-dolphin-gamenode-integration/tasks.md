# Tasks

Implementation is pending. This checklist owns the MVP execution detail; root TASKS.md retains
the broader roadmap. Keep both synchronized and record evidence before checking any item.

## 1. Host contract prerequisites

- [ ] 1.1 Record the SDK/host revision, required permissions and independent revision domains in protocol notes; verify against the public API and a selected-game fixture.
- [ ] 1.2 Demonstrate host-approved Dolphin executable acquisition and runtime ownership with an integration fixture; document any missing host UI/API as a blocking GameNode dependency before implementing launch.
- [ ] 1.3 Verify first-use proposal/review/execution and repeat-action intent semantics in a reference-host fixture; document when the intent is consumed and how approval state is recovered.

## 2. Identity and local metadata

- [ ] 2.1 Add explicit disc metadata with legacy field readers; verify catalog IDs never come from disc fields and document the field transition.
- [ ] 2.2 Implement copy/revision/multidisc identity and deduplication with host reconciliation fixtures preserving existing matches/history; document the migration strategy before changing record IDs.
- [ ] 2.3 Locate local Dolphin title files and implement bounded parsing and precedence; test user/system/English fallbacks, malformed or missing files and readable ROM/file-name fallbacks, then document supported data layouts.
- [ ] 2.4 Add explicit scan depth/entry-limit, symlink and overlapping-root regressions while retaining worker offloading; document remaining compressed-format limitations.

## 3. Persistence and catalog association

- [ ] 3.1 Add versioned provider cache records through persistent Storage and matching capabilities/manifests; verify granted/denied storage, restart, schema migration, cleanup, quota and revision-conflict fixtures.
- [ ] 3.2 Implement library-first matching and candidate resolution through the SDK; verify user choices survive, ambiguities stay unresolved and a private cache entry never authorizes launch, then document the matching flow.
- [ ] 3.3 Implement extension bookkeeping in its own Storage namespace; verify component/account isolation and absence of session handles, intents and credentials in stored records, and document recovery behavior.

## 4. Approved launch session

- [ ] 4.1 Capture host action context and intent and resolve the matched library entry/ROM handle; test missing target, unmatched entry, denied file access, missing ROM and readiness-only sessions without execution.
- [ ] 4.2 Resolve/register the approved owned runtime and stable content binding; add extension discovery permission, verify exact grants and independent binding/library revisions, and update permission documentation.
- [ ] 4.3 Propose and request immutable recipes using host consent and current intents; verify exact RPC sequence and no execution overrides or plugin process spawn, including refusals and expiry.
- [ ] 4.4 Reconcile durable IDs on restart/repeated Play; test no duplicate bindings/recipes and reapproval on changed runtime/content/grants, and document stale-state troubleshooting.

## 5. MVP packaging and acceptance

- [ ] 5.1 Build and verify both signed Linux x86_64 development packages with non-placeholder keys kept outside the repo; verify binary/manifest targets and permission parity, and document reproducible packaging commands and dependency notices.
- [ ] 5.2 Add CI for formatting, Clippy, workspace tests and package verification; verify the pipeline uses an external Cargo target and document SDK/API compatibility policy.
- [ ] 5.3 Run native GameCube and Wii discovery/matching/launch/repeated-Play acceptance in GameNode; record consent, cancellation, missing ROM, exit detection and playtime evidence with host/plugin revisions.
- [ ] 5.4 Run formatting, Clippy, workspace tests, git diff --check and strict OpenSpec validation; synchronize README, architecture, protocol and root backlog with actual completed behavior.

## Workflow follow-up

- Keep Windows releases, possible macOS/additional architectures, covers, saves and bundles in the root roadmap; each future release adds real binaries, manifest targets and native evidence.
- Archive this change only after its implementation and acceptance evidence exist; do not archive planning completion.
