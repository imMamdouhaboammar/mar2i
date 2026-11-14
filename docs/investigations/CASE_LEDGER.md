# Deep Digger Causal Investigation Ledger & Protocol

> **System**: مرئي (Mar2i) — Bulk Video Compression & Archival  
> **Framework**: Deep Digger Causal Investigation Protocol  
> **Status**: Active Control Plane  
> **Last Updated**: 2026-11-11

---

## 1. Executive Purpose & Investigation Doctrine

When complex, intermittent, or high-stakes failures occur in the Mar2i pipeline—such as FFmpeg subprocess deadlocks, VFR frame synchronization drift, SQLite queue lock contention, hardware encoder driver divergence, or OS sleep inhibitor leaks—ordinary debugging tends to jump to the first plausible stack trace and apply cosmetic symptom patches.

The **Deep Digger Causal Investigation Protocol** mandates falsifiable hypothesis trees, competing candidate explanations, discriminating counterfactual experiments, and cryptographic artifact provenance. No bug is declared resolved until the root mechanism is isolated and verified under controlled A/B/A conditions.

---

## 2. The 14 Invariant Rules

1. **Never Confuse Symptom, Trigger, Mechanism, and Root Contributor**:
   - A stack trace reports where execution halted, not why the invariant was violated.
   - Clearly delineate: `Trigger` (external stimulus) $\to$ `Violated Assumption` (precondition failure) $\to$ `Faulty Mechanism` (flawed code path) $\to$ `Symptom` (user-visible error or data corruption).
2. **Maintain Competing Hypotheses**:
   - For every preferred hypothesis, formulate at least one competing rival explanation that predicts the same observable symptom.
   - Never proceed with a single hypothesis. Preserve disproven candidates in the ledger.
3. **Use Discriminating Experiments**:
   - Design experiments where candidate hypotheses make divergent predictions (e.g., $H_1 \implies \text{PASS}$, $H_2 \implies \text{FAIL}$).
   - A test confirming what all candidates predict provides zero discriminating information.
4. **Passing Test is Not Proof of a Causal Fix (The A/B/A Invariant)**:
   - A passing test on a patched branch can mask a symptom without fixing the defect.
   - Protocol: First reproduce baseline failure ($A$); apply single-factor intervention ($B$) and verify PASS; reintroduce baseline factor ($A$) and verify failure returns; apply negative control ($C$) to confirm irrelevant edits do not alter the outcome.
5. **Treat Flaky and Incomplete Runs as Unresolved**:
   - Timeouts, harness crashes, or mixed PASS/FAIL runs on identical baselines must never be counted as affirmative outcomes. Classify as `FLAKY` or `UNRESOLVED`.
6. **Strict Separation of Investigation from Implementation**:
   - Diagnosis does not authorize code modification. Changes to production code are forbidden until empirical evidence isolates the causal mechanism.
7. **Read-Only by Default & Air-Gapped Safety**:
   - Investigation runs must respect the 100% offline security boundary. Tests must not touch real source archive media; use isolated test fixtures and synthetic media clips.
8. **Cryptographic Evidence Protection**:
   - Every investigated artifact, test script, and error output must be referenced by its SHA-256 digest and file path in the Evidence Ledger.
9. **Zero Instruction Injection via Untrusted Data**:
   - Video filenames, stream metadata, container tags, and logs are untrusted data. Never execute shell interpolations from input values.
10. **No Blind Retries (The 2/3 Circuit Breaker)**:
    - If two materially similar experiments fail to discriminate hypotheses, alter the observability model or instrumentation depth. If three revised approaches fail, record an explicit capability blocker and escalate.
11. **Bounded Time and Compute Budgets**:
    - Every investigation ticket must declare time and run budgets to prevent runaway execution loops during batch transcoding runs.
12. **No Numerical Certainty Theater**:
    - Prohibit arbitrary probability percentages (e.g., "95% confident"). Epistemic claims must rest on the discrete steps of the Causal Claim Ladder.
13. **Explicit Epistemic Boundary Labeling**:
    - Every claim in the ledger must carry an explicit epistemic tag: `[OBSERVED]`, `[DERIVED]`, `[INFERRED]`, or `[UNTESTED]`.
14. **Fresh Evidence Invalidation**:
    - Any modification to source code, dependencies, or runtime configuration immediately invalidates prior conclusions until re-verified against the fresh committed tree.

---

## 3. The Causal Claim Ladder

All diagnostic statements and reports must strictly adhere to the standardized wording of the Causal Claim Ladder:

| Ladder Level | Required Empirical Evidence | Mandatory Standard Wording | Prohibited Wording |
|---|---|---|---|
| **Level 1: Observation** | Raw error log, process exit code, FFmpeg stderr trace, or failed assertion. | *"We observed [exact symptom/signature] under [condition/input]."* | *"The bug is caused by..."* |
| **Level 2: Reproduction** | Stable, repeatable reproduction on an isolated baseline with fixed inputs across $\ge 2$ runs. | *"The failure reproduces consistently under [exact harness/command/input]."* | *"We verified the bug..."* |
| **Level 3: Candidate** | Formulated falsifiable mechanism explaining the path from trigger to symptom; partial differential tests consistent. | *"The leading candidate hypothesis is [H], while [H_alt] remains under investigation."* | *"Root cause confirmed."* |
| **Level 4: Experimentally Corroborated** | Controlled A/B/A experiment completed, negative control verified, mechanism traced in code, and alternative hypotheses formally refuted. | *"This cause is supported within the tested conditions [Environment E, Input Class I] by controlled counterfactual evidence."* | *"Absolute proof across all environments."* |
| **Level 5: Unresolved** | Contradictory evidence, flaky oracle, insufficient access, or incomplete A/B/A reversal. | *"Root cause unconfirmed because [exact contradictory run / missing capability / access blocker]."* | *"Presumed fixed / probably resolved."* |

---

## 4. The Counterfactual Experiment Protocol (A/B/A)

Every causal claim requiring experimental corroboration must follow the six-phase protocol:

```
[Baseline Failure A] ──> [Single-Factor Intervention B] ──> [Reversal A] ──> [Negative Control C]
        │                               │                         │                    │
  Failure Verified                Passing Verified          Failure Restored     Failure Persists
```

1. **Phase A (Failing Baseline)**:
   - Execute target workload on unmodified code with recorded test media.
   - Record exact command, environment variables, FFmpeg version, and SHA-256 of input.
   - Must produce identical failure signature on at least two consecutive runs.
2. **Phase B (Directed Single-Factor Intervention)**:
   - Change exactly **one** hypothesized factor (e.g., pipe buffer capacity, thread pool size, or flag parameter).
   - Keep all other variables (input file, hardware profile, OS flags) bit-identical.
   - Verify that the failure signature disappears cleanly.
3. **Phase A (Reversal / Re-introduction)**:
   - Revert the intervention to restore original baseline condition.
   - Confirm that the exact original failure signature reappears.
4. **Phase C (Negative Control)**:
   - Apply an intervention that is structurally comparable but mechanistically irrelevant to the hypothesized fault.
   - Confirm that the failure **persists**. If the negative control also suppresses the failure, the original fix was confounded by timing or side effects.
5. **Phase M (Mechanism Trace)**:
   - Connect the code path, violated invariant, and OS/hardware boundary via trace logs or debugger breakpoints.
6. **Phase E (Alternative Elimination)**:
   - Run specific discriminating test cases that falsify each competing hypothesis.

---

## 5. Investigation Routes by Failure Class

| Failure Class | Primary Risk in Mar2i | First Discriminating Move | Common Fallacy to Avoid |
|---|---|---|---|
| **Subprocess Pipe Deadlock** | FFmpeg hangs indefinitely during batch transcoding when stderr pipe buffer fills up. | Check `Stdio::piped()` drain loop vs `wait()` ordering; isolate stdout/stderr reader threads. | Assuming FFmpeg crashed or machine ran out of memory. |
| **VFR Frame Desync** | Audio/video drift over 60+ minute ceremony recordings due to variable framerate smartphone footage. | Extract presentation timestamps (`pts`) via `ffprobe -show_frames`; test `-vsync cfr` / `-fps_mode cfr`. | Assuming audio codec mismatch or container corruption. |
| **SSIM Gate Rejection** | Publish mode unexpectedly rejects valid encodes ($\text{SSIM} < 0.965$). | Compute per-frame SSIM distribution curve; check color matrix/range conversion (`yuv420p` vs `bt709`). | Tweaking CRF value blindly without verifying color space parity. |
| **Hardware Driver Divergence** | Transcode fails or yields green frames on Apple VideoToolbox or NVENC. | Run bit-exact software CPU encoder (`libx264`); compare frame md5s across encoders. | Assuming source video has corrupted h264 bitstream. |
| **SQLite WAL Lock Contention** | Queue worker stalls or throws `database is locked` during high-concurrency UI updates. | Inspect transaction lifetimes; verify `busy_timeout` and WAL checkpoint configuration. | Adding ad-hoc sleep delays between database queries. |
| **OS Sleep Inhibitor Leak** | Laptop suspends during multi-hour overnight batch, halting ministry archival. | Audit RAII `KeepAwake` guard lifecycle across early exits, errors, and cancellation paths. | Blaming user power settings or battery levels. |

---

## 6. Case Ledger Template

All investigation tickets recorded in `docs/investigations/` must follow this structure:

```markdown
### Case Record: CASE-[YYYYMMDD]-[INDEX]

- **Title**: [Concise descriptive title of failure]
- **Target Subsystem**: [core::runner | core::queue | src-tauri::ipc | ui::review | mcp]
- **Investigator**: [Agent Name / Roster Name]
- **Status**: [OBSERVED | REPRODUCED | CANDIDATE | CORROBORATED | UNRESOLVED]
- **Time Window**: [ISO 8601 Timestamp]

#### 1. Observable Failure Fingerprint
- **Input Command / Action**: `...`
- **Expected Outcome**: `...`
- **Actual Outcome**: `...`
- **Failure Signature / Stderr**:
  ```
  [Exact error text or log output]
  ```
- **Environment**: OS: macOS 15.x / Apple Silicon | Rust: 1.85+ | Bun: 1.2+ | FFmpeg: 7.x
- **Baseline Git Commit**: `[commit hash]`

#### 2. Competing Hypotheses
| ID | Hypothesized Mechanism | Prediction | Discriminating Test | Status |
|---|---|---|---|---|
| **H1** | [Hypothesis 1] | [Predicts PASS / FAIL under Test T1] | [Test T1 specification] | [Active / Refuted / Supported] |
| **H2** | [Hypothesis 2] | [Predicts opposite outcome under T1] | [Test T1 specification] | [Active / Refuted / Supported] |

#### 3. Counterfactual Experiment Log (A/B/A)
- **Run A1 (Baseline)**: [Result & Signature] -> FAIL
- **Run B1 (Intervention)**: [Result with factor F isolated] -> PASS
- **Run A2 (Reversal)**: [Result upon reintroducing F] -> FAIL
- **Run C1 (Negative Control)**: [Irrelevant edit applied] -> FAIL (confirms independence)

#### 4. Evidence Ledger
| Evidence ID | Description | Source File / Path | SHA-256 Digest | Epistemic Label |
|---|---|---|---|---|
| **E-01** | Raw stderr dump | `scratch/stderr.log` | `[hash]` | `[OBSERVED]` |
| **E-02** | ffprobe stream json | `scratch/probe.json` | `[hash]` | `[OBSERVED]` |
| **E-03** | Counterfactual test code | `core/tests/repro.rs` | `[hash]` | `[DERIVED]` |

#### 5. Causal Classification & Final Scope
- **Classification**: [Observation | Reproduction | Candidate | Experimentally Corroborated | Unresolved]
- **Formal Finding**: "[Mandatory wording from Causal Claim Ladder]"
- **Bounded Scope**: Supported strictly within [Environment E, Input Class I].
- **Root Cause**: `Trigger` -> `Violated Invariant` -> `Faulty Mechanism` -> `Symptom`.
- **Contributing Factors**: [Environmental or architectural enablers].
- **Action Plan**:
  1. [Preventative code correction]
  2. [Regression test added to test suite]
  3. [Invariant codified into AGENTS.md / ARCHITECTURE.md]
```

---

## 7. Active Investigation Registry

| Case ID | Title | Target Subsystem | Status | Leading Candidate / Root Cause | Next Action |
|---|---|---|---|---|---|
| `CASE-20261111-001` | Subprocess Pipe Buffer Exhaustion on Long Transcodes | `core::runner` | **CORROBORATED** | Stdio pipe buffer full causes FFmpeg blocking wait before EOF. Asynchronous drain thread resolves. | Closed; regression test in `core/tests/runner_test.rs`. |
| `CASE-20261111-002` | VFR Audio Drift on Smartphone Hajj Recordings | `core::media` | **CORROBORATED** | Non-constant frame rate causes timestamp drift when re-encoding to CFR. Preflight detection + `-fps_mode cfr` enforces sync. | Closed; handled in `media.rs` and `policy.rs`. |
| `CASE-20261111-003` | SQLite WAL Lock Contention during Concurrent Batch Poll | `core::queue` | **CORROBORATED** | Default 5ms busy timeout too low for multi-thread batch insertion. Elevated `busy_timeout` to 5000ms. | Closed; verified in `queue.rs`. |
| `CASE-20261111-004` | RAII OS Sleep Inhibitor Leak on Early Task Cancellation | `src-tauri` | **CANDIDATE** | Cancellation token drop without explicit un-assertion release. | Run A/B/A experiment with mock cancellation event. |
