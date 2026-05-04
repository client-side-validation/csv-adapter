# Repository Split Strategy
## CSV Adapter — When, What, and How to Split

---

## The Single Rule That Governs Everything

**Never split what must change in the same commit.**

The test: if changing file X requires changing file Y to keep the build green, 
X and Y belong in the same repository. Splitting them creates cross-repo PRs, 
version negotiation overhead, and broken CI windows between commits.

Applied to your codebase, this immediately produces two permanent groups:

```
GROUP A — Must always change together (NEVER split)
  csv-adapter-core
  csv-adapter-bitcoin
  csv-adapter-ethereum
  csv-adapter-sui
  csv-adapter-aptos
  csv-adapter-solana
  csv-adapter-store
  csv-adapter-keystore
  csv-adapter
  csv-schemas (future)
  csv-cli

GROUP B — Has independent release lifecycle (CAN split when stable)
  csv-wallet
  csv-explorer
  typescript-sdk (future, different language — split from day 1)
  csv-mcp-server (future, split from day 1)
```

The reason `csv-cli` stays in Group A: every change to `AnchorLayer`, 
`FullChainAdapter`, or `protocol_version.rs` is immediately tested through the CLI 
commands. The CLI is the protocol's integration test surface. Separating it would 
create a constant stream of "update CLI to match new protocol" cross-repo PRs.

---

## Current State Assessment

All crates are at `version = "0.4.0"`. Two crates already correctly signal 
non-library status: `csv-explorer/Cargo.toml` and `csv-wallet/Cargo.toml` both 
have `publish = false`.

`PROTOCOL_VERSION_STR` in `protocol_version.rs` is a build-generated constant 
(the `{}` placeholder is filled at compile time). This is the canonical gate.

The workspace uses `resolver = "2"` and `workspace.package.version` — a single 
version shared by all crates. This is correct for the monorepo phase but must 
change before any crate is published to crates.io independently.

---

## The Proposed GitHub Organization Layout

```
github.com/{your-org}/
  csv-protocol/          ← Main monorepo. Rust workspace. NEVER splits.
  csv-wallet/            ← Split when: Phase 3 done (design system stable)
  csv-explorer/          ← Split when: Phase 1 testnet tests pass
  typescript-sdk/        ← Separate from day 1 (different language)
  csv-mcp-server/        ← Separate from day 1 (different language/target)
  .github/               ← Org-level: CODE_OF_CONDUCT, SECURITY.md, issue templates
```

The name `csv-protocol` for the main repo signals clearly: this is the protocol, 
not an app. Everything else is built on top of it.

---

## Split Decision Table

| Repo | Split when | Gate condition | Risk of splitting too early |
|------|-----------|----------------|----------------------------|
| **csv-wallet** | After Phase 3 | Design system done, Tailwind CDN removed, wallet state unified | Mid-development splits fragment PRs across repos constantly |
| **csv-explorer** | After Phase 1 testnet pass | Integration tests green on signet/testnet | Explorer Cargo.toml uses `path = "../csv-adapter-core"` — must become crates.io dep first |
| **typescript-sdk** | When you start building it (Phase 6) | Language boundary is the natural split — start separate | N/A — doesn't exist yet |
| **csv-mcp-server** | When you start building it (Phase 6) | Same | N/A — doesn't exist yet |
| **csv-schemas** | After Phase 4 | First 2 schemas deployed on testnet | Too early = unstable API, external devs importing a moving target |
| **Protocol core** | Never | — | — |

---

## The Version Gate: What "Ready to Split" Means Precisely

Before any Group B repo splits out, the following must be true:

**Gate 1 — Protocol API is stable**  
`csv-adapter-core` version is `1.0.0`.  
All items marked `🔒 Stable` in `lib.rs` have not changed for 60 days.  
`PROTOCOL_VERSION` constant is `1.0.0`.  
The 5 critical bugs from the main plan are fixed.

**Gate 2 — crates.io publishing works**  
Every Group A crate is published to crates.io in dependency order:
```
1. csv-adapter-core          (no workspace deps)
2. csv-adapter-keystore      (depends on core)
3. csv-adapter-store         (depends on core)
4. csv-adapter-bitcoin       (depends on core)
5. csv-adapter-ethereum      (depends on core)
6. csv-adapter-sui           (depends on core)
7. csv-adapter-aptos         (depends on core)
8. csv-adapter-solana        (depends on core)
9. csv-adapter               (depends on all above)
10. csv-schemas              (depends on core)
```
After publishing, verify each crate installs cleanly: `cargo add csv-adapter-core`.

**Gate 3 — Application repo Cargo.toml migrated to crates.io deps**  
`csv-wallet/Cargo.toml` changes from:
```toml
csv-adapter = { path = "../csv-adapter", ... }
```
to:
```toml
csv-adapter = { version = "^1.0", ... }
```
This is the physical act of severing the path dependency. The repo split follows 
immediately after — not before.

**Current blocker on Gate 1:**  
`csv-wallet/Cargo.toml` directly imports `csv-adapter-solana = { path = "../csv-adapter-solana" }`.  
This violates the dependency rule from the main plan (Phase 2.2): wallet must only 
import through `csv-adapter::ChainFacade`, not raw chain adapters.  
Fix this first — it is blocking crates.io publishing for the wallet split.

---

## Versioning Strategy After Split

### Protocol workspace: strict semver

```
0.x.y  — current phase (breaking changes allowed with minor bump)
1.0.0  — stable release, the split gate
1.x.y  — additive changes only, no breaking changes to Stable API
2.0.0  — next breaking change (requires updating all application repos)
```

Each Group A crate publishes independently to crates.io but the workspace `Cargo.toml` 
enforces they all share the same version number. A script in CI verifies this:

```bash
# .github/workflows/version-check.yml
# All protocol crates must share the same version
VERSIONS=$(cargo metadata --no-deps --format-version 1 | \
  jq '[.packages[] | select(.name | startswith("csv-adapter")) | .version] | unique')
if [ $(echo $VERSIONS | jq length) -ne 1 ]; then
  echo "ERROR: Protocol crates have divergent versions"
  exit 1
fi
```

### Application repos: independent semver, compatibility matrix

```
csv-wallet v1.x.y requires csv-adapter "^1.0"   (compatible with any 1.x)
csv-wallet v2.x.y requires csv-adapter "^2.0"   (requires protocol 2.x)

csv-explorer v1.x.y requires csv-adapter-core "^1.0"
```

Each application repo `README.md` carries a compatibility table:

```markdown
## Compatibility

| csv-wallet | csv-adapter | csv-adapter-core |
|------------|-------------|-----------------|
| 1.0.x      | 1.0.x       | 1.0.x           |
| 1.1.x      | 1.1.x       | 1.0.x - 1.1.x  |
| 2.0.x      | 2.0.x       | 2.0.x           |
```

### TypeScript SDK: npm semver, wire format version

The TypeScript SDK does not import the Rust crates. It reimplements (or WASM-wraps) 
the wire format. It declares compatibility against the protocol wire format version, 
not the Rust crate version:

```json
// typescript-sdk/package.json
{
  "name": "@csv-protocol/sdk",
  "version": "1.0.0",
  "peerDependencies": {},
  "csvProtocolWireFormat": "1"
}
```

The `PROTOCOL_VERSION` constant in `csv-adapter-core/src/protocol_version.rs` is 
the wire format version. TypeScript SDK documents which wire format it implements.

---

## How to Execute Each Split

### Split 1: csv-explorer (first to split, after Phase 1)

**Why first:** Explorer has a Docker deployment with its own release cycle. 
It doesn't change when the wallet changes. Its crate structure is already 
self-contained under `csv-explorer/`.

**Steps:**
```bash
# 1. In the new repo: git filter-repo to extract history
git clone csv-protocol csv-explorer-repo
cd csv-explorer-repo
git filter-repo --path csv-explorer/ --path-rename csv-explorer/:./

# 2. Update Cargo.toml: replace path deps with crates.io versions
# csv-adapter-core = { path = "../csv-adapter-core" }
# → csv-adapter-core = "1.0.0"

# 3. Add release CI to new repo
# .github/workflows/release.yml — docker build + push on tag

# 4. Update csv-protocol to remove csv-explorer from workspace members
# Cargo.toml: remove "csv-explorer" from [workspace].members

# 5. Tag csv-protocol with "explorer-split-v1.0.0" 
#    so history is traceable
```

**What stays in csv-protocol:** nothing. csv-explorer is fully extracted.

### Split 2: csv-wallet (after Phase 3)

**Why second:** Wallet WASM build has different toolchain requirements. 
Releases are user-facing (semantic versioning visible to users). 
Different CI: requires `wasm-pack` or `dx build --platform web`.

**Steps:** Same `git filter-repo` pattern.

Additional step — remove the direct chain adapter import from wallet:
```toml
# BEFORE (blocks split):
csv-adapter-solana = { path = "../csv-adapter-solana", optional = true }

# AFTER (enables split):
# Deleted. All chain operations go through csv-adapter::ChainFacade.
# ChainFacade already handles Solana internally.
```

**New CI for csv-wallet repo:**
```yaml
# .github/workflows/build.yml
- run: dx build --platform web --release
- run: wasm-opt -Oz pkg/*.wasm -o pkg/optimized.wasm
# No cargo test --workspace — tests run against crates.io deps
```

### Split 3: typescript-sdk (start directly in new repo)

Never existed in the monorepo. Create fresh:
```bash
mkdir typescript-sdk
cd typescript-sdk
npm init @csv-protocol/sdk
# Pin csv-adapter-core wire format version in README
```

If WASM compilation of `csv-adapter-core` is needed for proof verification:
```bash
# In csv-protocol repo:
cargo build --target wasm32-unknown-unknown -p csv-adapter-core
wasm-bindgen --target web --out-dir ../typescript-sdk/src/wasm/
```
This produces the WASM artifact that the TypeScript SDK imports as a package asset. 
The protocol repo publishes WASM artifacts as GitHub releases. TypeScript SDK 
downloads them at `npm install` time via a postinstall script.

---

## What Remains in csv-protocol Forever

```
csv-protocol/
  csv-adapter-core/         ← Protocol primitives
  csv-adapter-bitcoin/      ← Bitcoin adapter
  csv-adapter-ethereum/     ← Ethereum adapter
  csv-adapter-sui/          ← Sui adapter
  csv-adapter-aptos/        ← Aptos adapter
  csv-adapter-solana/       ← Solana adapter
  csv-adapter-store/        ← Storage layer
  csv-adapter-keystore/     ← Key management
  csv-adapter/              ← Unified facade
  csv-schemas/              ← Contract schema library (Phase 4)
  csv-cli/                  ← Protocol CLI (stays — integration test surface)
  examples/                 ← Protocol usage examples
  docs/                     ← Protocol documentation
  .github/workflows/
    ci.yml                  ← Full workspace test + clippy + audit
    release.yml             ← Publish to crates.io on tag
    wasm-artifacts.yml      ← Build WASM for TypeScript SDK to consume
```

---

## Anti-Patterns to Avoid

**Anti-pattern 1: Splitting before Gate 1 (protocol API stable)**  
If `csv-adapter-core` is still changing its public API, every split creates 
constant cross-repo version bumps. Each breaking change in core requires 
a PR in csv-wallet, csv-explorer, and typescript-sdk simultaneously. 
That is exactly the problem splitting was supposed to solve — you've made it worse.

**Anti-pattern 2: Splitting by "size" rather than "release cadence"**  
The explorer has a lot of code. The wallet has a lot of code. Neither should split 
because it's large. They split because their deployments are independent. 
A 5000-line crate with the same release cadence as core stays in the monorepo.

**Anti-pattern 3: Splitting csv-adapter-core from the chain adapters**  
Some projects split the "interface" crate from "implementation" crates into 
separate repos. Do NOT do this. A change to `AnchorLayer` in csv-adapter-core 
requires updating all 5 chain adapters immediately. They must be in one atomic commit.

**Anti-pattern 4: Publishing to crates.io before the split**  
Publishing to crates.io does not require a repo split. You can publish 
`csv-adapter-core` to crates.io from the monorepo today (once the API is stable). 
The repo split is a separate decision from the crates.io publishing decision.

**Anti-pattern 5: Using git submodules to "split" while keeping them coupled**  
Submodules give you the worst of both worlds: the friction of separate repos 
with the coupling of a monorepo. Don't do it.

---

## Recommended Timeline

```
Now — Phase 1 (protocol correctness):
  Stay in monorepo. Fix BUG-01 to BUG-05. Wire validator. Wire real seals.
  
After Phase 2 (codebase control):
  Fix the csv-wallet direct Solana import (blocks future split).
  Prepare workspace for independent versioning:
    - Give each Group A crate its own version field alongside workspace.package.version
    - Add the version-consistency CI check
    
After Phase 3 (wallet design done):
  Bump csv-adapter-core to 1.0.0-rc.1.
  Publish Group A crates to crates.io for testing.
  
After Phase 1 testnet tests green (≈ same time as Phase 3):
  Execute Split 1: csv-explorer → own repo.
  csv-explorer/Cargo.toml now uses crates.io deps.
  
After Phase 3 complete (wallet design system stable):
  Execute Split 2: csv-wallet → own repo.
  csv-wallet/Cargo.toml now uses crates.io deps.
  Bump csv-adapter-core to 1.0.0.
  
Phase 6 start:
  Create typescript-sdk/ as new repo from day 1.
  Create csv-mcp-server/ as new repo from day 1.
  
Phase 4 (contract schemas deployed on testnet):
  Publish csv-schemas to crates.io.
  Consider splitting to own repo if external contributors start using it.
```

---

## The Precise Split Checklist

Before executing any split, verify all of these:

```
Protocol readiness:
  [ ] csv-adapter-core is at version 1.0.0 (or 1.0.0-rc.1 for explorer split)
  [ ] All 🔒 Stable API items in lib.rs unchanged for 60 days
  [ ] No open PRs that change AnchorLayer, FullChainAdapter, or SealRef types
  [ ] PROTOCOL_VERSION constant matches the crates.io published version

crates.io readiness:
  [ ] All Group A crates published to crates.io in dependency order
  [ ] `cargo add csv-adapter-core` works from an empty project
  [ ] docs.rs renders correctly for all published crates

Application repo readiness:
  [ ] The splitting repo's Cargo.toml has been migrated from path deps to crates.io deps
  [ ] CI passes with crates.io deps (not path deps)
  [ ] The direct csv-adapter-solana import in csv-wallet is removed (wallet only)
  [ ] git filter-repo produces a repo that builds independently

Post-split:
  [ ] Original monorepo removes the crate from [workspace].members
  [ ] Original monorepo CI still passes (no path dep left pointing to extracted crate)
  [ ] New repo has its own CI, release workflow, and README
  [ ] New repo README documents compatibility with csv-adapter-core version
  [ ] A git tag on csv-protocol marks the split point for history traceability
```
