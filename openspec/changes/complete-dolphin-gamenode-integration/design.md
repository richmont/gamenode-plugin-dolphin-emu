# Design

## Context

See proposal.md for motivation. The workspace contains a provider with synchronous scanning
offloaded through `spawn_blocking` and a launch extension with tested pure recipe builders but no
session implementation. The 10 existing unit tests passed during the initial analysis; no native
end-to-end Dolphin acceptance is established. Only the public sibling SDK is a dependency.

## Goals / Non-Goals

**Goals:** Preserve host identity and consent while adding offline metadata resolution, matching,
restart-safe state and the Linux MVP launch flow.

**Non-Goals:** Shared private storage between components, copying host internal types, a new
central game database, direct process management or claiming other platforms from a cross-build.
See the root TASKS.md for post-MVP enrichment and packaging work.

## Decisions

1. **Separate identity domains.** Introduce explicit disc metadata and read legacy `fields.game_id`
   only as disc identity. Resource requests take the host-selected catalog ID and installation.
   Copies and multidisc entries must remain independently addressable. Do not reuse the six-byte
   disc ID as a universal key. Preserve existing library identities/history during migration;
   establish the host reconciliation behavior in fixtures before changing provider record IDs.

2. **Use local text title databases.** Follow user mapping, localized system mapping, English,
   ROM metadata and file-name precedence. Resolve Dolphin system/user data directories separately
   from its configuration files; parse bounded input on workers. Cache source/language and a
   source fingerprint so changed local metadata can invalidate cached titles. Direct binary
   GameListCache parsing would couple the plugin to emulator serialization and is deferred.
   References and the title-versus-synopsis distinction are in docs/architecture.md.

3. **Keep catalog matching authoritative in GameNode.** Read the existing match first, then search
   through the public API and assign only a resolved candidate. Do not override user choices or
   infer a match from a cached title alone. Ambiguities use the host's matching flow; the extension
   reports an unresolved association before launch. No new matching UI is assumed inside the plugin.

4. **Use host Storage, not a separate database.** Store small schema-versioned records for observed
   metadata and matching hints in the provider; store durable launch IDs/revisions in the extension.
   Use opaque identity-derived keys, persistent durability, idempotency keys and optimistic storage
   revisions. Bound values below 1 MiB and total logical usage below the 16 MiB quota. Cache only
   discovered titles, evict rebuildable metadata first and never discard host matches/history.
   Denied persistence must be explicit; loss of a cache cannot authorize a new launch. Read shared
   game state through the library, since package IDs have isolated namespaces even in a bundle.

5. **Host-approved runtime setup is a prerequisite.** Prefer the host approval UI, as already
   specified in the backlog. Demonstrate an approved executable file handle and an owned runtime
   usable for content registration before session implementation. If the public host flow cannot
   provide this, record the precise missing host contract as separate GameNode work; do not assume
   a shared catalog row grants execution. A prefix-based alternative requires documented consent
   and equivalent fixtures before adoption.

6. **Reconcile launch state per selected action.** Capture the current intent before session setup;
   resolve the library entry, approved ROM handle and runtime; create or update content using its
   own revision; propose/recover an immutable recipe; request execution with the current library
   revision and approved recipe revision. Establish first-use review/intent behavior in the SDK
   fixture, since proposal is not itself process execution. Guard initialization and readiness
   paths against launch. Persist no file handles, credentials or intents. Stale references require
   rereads/reapproval, not blind retries or direct spawning.

7. **Release components first.** Two Linux x86_64 packages are sufficient for MVP acceptance.
   Publisher placeholders must be replaced for packaging; keys remain outside version control.
   Windows is planned and macOS/other architectures optional, each with explicit targets and
   native validation. Bundles depend on the host proposal and do not block component acceptance.

## Risks / Trade-offs

- [Host runtime approval path unverified] → Gate launch implementation on a concrete approved
  handle/ownership fixture; track missing host work separately.
- [Identity migration changes discovery keys] → Test copies, multiple discs and existing history
  with the real host reconciliation contract before changing keys.
- [Dolphin data paths vary] → Missing local databases fall back offline; test native, portable and
  Flatpak layouts independently instead of equating configuration discovery with launch support.
- [Schema drift or user cleanup] → Version cache records, reject unknown versions safely and
  rebuild metadata while revalidating durable references against the host.
- [Intent expires during setup] → Report expiry and require a fresh host action rather than
  inventing/reusing a token.
- [Host docs can lag implementation] → Pin the SDK/host revision in acceptance evidence and verify
  permissions and revision semantics with fixtures.

## Migration Plan

1. Add compatibility fixtures and versioned storage readers before writing new metadata fields.
2. Implement local metadata/matching and launch state with corresponding capability/manifest updates.
3. Validate persistent restart and existing library identity retention. Keep migration additive;
   do not destructively remove legacy fields or host records in the MVP.
4. Package development builds and run native acceptance before release. On rollback, use host
   package rollback, retain library/history and tolerate unsupported cache schema by rebuilding
   disposable metadata. Never automatically execute a recipe solely because cache state survived.
