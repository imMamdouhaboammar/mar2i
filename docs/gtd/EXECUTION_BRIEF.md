# Execution Brief: مرئي (Mar2i) — Bulk Video Compression & Archival System

## Outcome
- **Problem**: Saudi Ministry of Hajj & Umrah media operations handle massive volumes of high-bitrate video footage across Hajj and Umrah seasons under strict storage, network, and privacy constraints. Media processing must remain 100% offline, original files must never be modified, and archival/publishing integrity must be guaranteed.
- **Desired outcome**: Offline desktop bulk video compression and archival for Saudi Ministry of Hajj & Umrah media operations.
- **Actor**: Saudi Ministry of Hajj & Umrah media operations team and archive custodians
- **Status**: ready
- **Domain**: software

---

## Scope

### In
- Offline desktop hybrid application built with Tauri v2, Rust core (`mar2i-core`), SQLite WAL job queue, and TypeScript/Vite RTL UI.
- Safe directory discovery rejecting symlinks, preventing directory traversal, and validating canonical root boundaries.
- Dual processing modes: Publish Mode (H.264/MP4, SSIM ≥ 0.965 quality gate, copy-through for already efficient codecs) and Archive Mode (lossless FFV1/MKV with decoded-frame hash verification `framemd5`, byte-copy with SHA-256 for non-raw assets).
- Strict hardware acceleration policy separation: GPU hardware acceleration allowed only for Publish mode; deterministic, reproducible CPU encoding mandated for Archive mode.
- Zero-leak RAII OS Sleep Inhibitor (`KeepAwake`) preventing workstation sleep during long-running batch jobs.
- Stratified multi-sample empirical size estimation predicting GB savings before transcoding begins.
- Read-only original media guarantee: source files are immutable sacred archives.
- SQLite WAL persistent queue with pause, resume, and cancellation across worker crashes.
- Exportable audit reports in CSV and HTML formats with SHA-256 and SSIM proof metrics.
- Optional local read-only MCP stdio interface strictly constrained to `MAR2I_MCP_ALLOWED_ROOTS`.

### Out
- Cloud-based transcoding, external storage uploads, remote database sync, or third-party telemetry sinks (strictly 100% offline).
- In-place mutation or deletion of source media files.
- Lossy re-encoding of HDR streams, non-AAC multichannel audio, or complex subtitle tracks in Publish mode.
- Unconstrained filesystem access or write permissions for MCP tools.

### Constraints
- 100% local, air-gapped / offline operation — zero network egress or telemetry.
- Read-only source media contract: input files must never be modified, moved, or deleted.
- Hardware acceleration discipline: GPU for speed in publishing; CPU for exact mathematical determinism in archival.
- Never overwrite destination files without explicit operator consent; reject 0-byte or oversized outputs.
- Bun package manager mandatory for JavaScript/TypeScript; Rust edition 2024; Tauri v2 IPC architecture.
- Arabic RTL primary layout; WCAG 2.2 accessibility standards.

---

## Knowledge Ledger

### Facts
- Video files from Hajj seasons span diverse containers (MOV, MP4, MKV, AVI) and codecs (ProRes, DNxHD, H.264, HEVC).
- Tauri v2 IPC bridges the TypeScript WebView to the native Rust engine `mar2i-core`.
- FFmpeg and ffprobe binaries are required locally for stream inspection and transcoding.
- `framemd5` ensures lossless decoded-frame equality between raw sources and FFV1 archives.
- SQLite in WAL mode ensures atomic, crash-resilient queue persistence across process crashes.

### Assumptions
- Operator machines have sufficient local or attached storage for transcoded output files.
- Operator workstations run modern macOS or Windows environments with multi-core CPUs.
- GPU hardware encoding drivers may exhibit slight frame discrepancies across vendors, reinforcing the CPU-only policy for archival.

### Unknowns
- Exact percentage of legacy interlaced footage vs progressive footage in Ministry archive backlog.
- Thermal limits of field laptops during multi-hour continuous batches in hot field environments.

---

## Decisions
| Decision | Rationale | Reversible |
|---|---|---|
| Strict hardware acceleration separation: GPU for Publish, CPU for Archive | GPU encoders lack bit-exact determinism across drivers; archival requires mathematically reproducible frames | false |
| Verification threshold SSIM ≥ 0.965 for Publish mode | Guarantees indistinguishable visual fidelity for broadcast and social distribution | true |
| SQLite WAL queue persistence | Crash resilience and zero database locking contention between UI polling and worker transcode threads | false |
| Read-only source media contract | Protects irreplaceable historical religious and cultural footage from accidental destruction | false |
| Stdio-only MCP interface with path grants | Prevents unauthorized filesystem exposure while enabling local AI workflow integration | true |

---

## Open Decisions
- None (all blocking architectural decisions resolved).

---

## Workstreams
| Workstream | Outcome | Dependencies |
|---|---|---|
| Core Engine (`mar2i-core`) | Safe discovery, ffprobe media inspection, policy engine, runner, SQLite queue, worker, and reporter | None |
| Desktop Shell (`src-tauri`) | Tauri v2 IPC bridge, sleep inhibitor, and FFmpeg bundle discovery | Core Engine (`mar2i-core`) |
| Frontend Application (`src`) | Arabic RTL UI, reactive batch progress, codec options cards, and A/B visual preview | Desktop Shell (`src-tauri`) |
| Verification & Security Architecture | Zero-mock integration tests, SSIM gate checks, framemd5 verification, and threat model mitigations | Core Engine (`mar2i-core`), Desktop Shell (`src-tauri`) |

---

## Deliverables
- `core/` — Production Rust crate `mar2i-core` with discovery, media probe, policy, runner, queue, worker, and report modules.
- `src-tauri/` — Native Tauri v2 desktop application shell with IPC command routing and RAII sleep inhibitor.
- `src/` — Accessible Arabic RTL-first TypeScript/Vite UI with real-time job monitoring and visual comparison.
- `docs/security/THREAT_MODEL.md` — STRIDE security evaluation and attack surface inventory.
- `docs/gtd/EXECUTION_BRIEF.md` & `docs/gtd/execution-brief.json` — Formal GTD work model and schema specification.
- `docs/investigations/CASE_LEDGER.md` — Deep Digger causal investigation protocol and case ledger.

---

## Risks
- Host system sleep or thermal throttling interrupting multi-hour batches (mitigated by RAII KeepAwake and resource profiles).
- Video container edge cases (VFR, rotational metadata, corrupt moov atoms) causing transcode stalls (mitigated by ffprobe preflight).
- Storage exhaustion mid-batch when compressing massive 4K raw archives (mitigated by stratified size prediction and disk headroom check).

---

## Verification

### Success criteria
- All Rust core integration tests pass cleanly (`cargo test --manifest-path core/Cargo.toml`).
- All TypeScript unit tests pass cleanly (`bun test`).
- SSIM evaluation correctly gates publish encoding and rejects outputs below 0.965.
- Archive mode validates lossless frame identity via `framemd5` on FFV1 transcode.
- Discovery engine rejects symlinks and directory traversal attacks.
- MCP server denies filesystem access outside `MAR2I_MCP_ALLOWED_ROOTS`.

### Evidence
- `core/tests/` unit and integration test outputs with 100% green assertions.
- `src/data.test.ts` test results.
- `docs/security/THREAT_MODEL.md` verified controls.
- Verified execution brief schema validation output (`gtd validate-brief`).

---

## Blockers
- None.

---

## Next executable action
- Execute end-to-end batch transcode validation tests and audit report generation.

---

## GTD Operating Modes Protocol

The Mar2i development lifecycle strictly navigates through the 8 GTD Operating Modes. The active mode is determined dynamically by the current blocker:

| Mode | Trigger Signal | Active Focus & Operations in Mar2i | Completion Artifact |
|---|---|---|---|
| **1. Clarify** | Desired outcome, media profile, or domain requirements are materially ambiguous | Clarify ministry user persona, input container profiles (ProRes, DNxHD, H.264), and operational constraints (air-gapped workstations). | Resolved intent in `EXECUTION_BRIEF.md`. |
| **2. Research** | A discoverable technical fact or dependency constraint is missing | Benchmark FFmpeg flags, investigate Apple VideoToolbox / NVENC capabilities, evaluate OS sleep-inhibition APIs (`IOPMAssertionCreateWithName`, `SetThreadExecutionState`). | Documented Facts and citations in Knowledge Ledger. |
| **3. Decompose** | Scope contains multiple independently deliverable subsystems | Split system into `mar2i-core` (Rust engine), `src-tauri` (desktop shell), `src` (RTL UI), and `mcp` (local tools). | Decoupled workstream DAG with explicit dependency edges. |
| **4. Decide** | Competing architectural options have divergent system consequences | Evaluate CPU vs GPU for archival, SQLite WAL vs in-memory state, Tauri IPC protocol boundaries. | Recorded entries in Decisions Table with reversibility tags. |
| **5. Validate** | Feasibility, risk, or performance assumptions are unproven | Validate stratified size estimation accuracy across test clips; measure SSIM calculation latency overhead. | Empirical validation logs and benchmark reports. |
| **6. Model** | Context is sufficient but work structure is not executable yet | Model SQLite job state machine (`Pending` → `Probing` → `Transcoding` → `Verifying` → `Completed` / `Failed`); formalize IPC contracts. | Fully specified schemas, type definitions, and test vectors passing the Ready Gate. |
| **7. Execute** | Ready Gate passes and implementation is authorized | Implement Rust modules and TypeScript components adhering to Anti-Slop and Clean Code Guard standards. | Tested, compiling, reviewed production code. |
| **8. Verify** | Deliverables exist and require falsifiable proof | Execute automated test suites (`cargo test`, `bun test`), run SSIM gates, verify `framemd5` hashes, audit security controls. | Concrete verification evidence and passing Definition of Done. |

---

## Ready Gate Criteria (Definition of Ready)

Before moving any workstream or feature from `modeling` to `executing`, all 10 Ready Gate criteria must be satisfied:

1. **Understandable Desired Outcome**: The problem and targeted deliverable are unambiguous and documented.
2. **Identified Actor / Beneficiary**: Explicit target users (Ministry of Hajj & Umrah media staff) and roles are identified.
3. **Bounded Scope**: In-scope items and explicit out-of-scope boundaries are defined without scope creep.
4. **Visible Critical Constraints**: Air-gapped offline operation, read-only media guarantee, and hardware acceleration rules are enforced.
5. **Resolved Blocking Decisions**: Zero open decisions remain on the critical execution path.
6. **Explicit Critical Assumptions**: Assumptions are recorded visibly with known downside bounds.
7. **Transparent Unknowns**: Any blocking unknown is elevated to an explicit Blocker rather than hidden.
8. **Checkable Success Criteria**: Every deliverable has concrete, automated, or falsifiable acceptance tests.
9. **Single Next Executable Action**: The immediate next step is explicitly defined and directly executable.
10. **Freshness & Dependency Evidence**: Local tools, FFmpeg binaries, and Rust toolchains are verified in the active environment.

---

## Non-Negotiables & System Invariants

1. **Zero Cloud Leakage (Air-Gapped Invariant)**: The application must never bind external network listeners, emit telemetry, or transmit media or metadata outside the local workstation.
2. **Sacred Source Media (Read-Only Invariant)**: Source files are treated as immutable sacred archives. No operation may write to, rename, truncate, or delete original source files.
3. **Hardware Policy Separation**: GPU acceleration is strictly reserved for high-throughput Publishing. Archival transcoding demands deterministic, bit-exact CPU processing (`libx264` / `ffv1`).
4. **Mathematical Verification Invariant**: No transcode job transitions to `Completed` without passing verification: SSIM ≥ 0.965 for Publish, decoded-frame hash identity (`framemd5`) for Archive FFV1, or cryptographic SHA-256 verification for byte-copied streams.
5. **Crash-Durable Persistence**: All job states and queue mutations must commit through SQLite WAL transactions to prevent queue corruption during unexpected process terminations or power cuts.
6. **Anti-Slop & Direct Payload Invariant**: Ban speculative fluff, placeholder mocks in production paths, and superficial play-by-play code comments.
