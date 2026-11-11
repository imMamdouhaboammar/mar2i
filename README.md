# مرئي (Mar2i)

Offline desktop application for bulk video compression and archival — built for video-operations teams managing large media collections.

**Stack:** TypeScript + Vite (UI) · Rust core library · Tauri v2 desktop shell  
**Target users:** Saudi Ministry of Hajj and Umrah media staff  
**Locale:** Arabic RTL primary; English planned

---

## Architecture

```
mar2i/
├── src/              # TypeScript UI — Vite, vanilla TS, Arabic RTL
│   ├── main.ts       # Rendering engine and event dispatch (no framework)
│   ├── data.ts       # Pure data types, codec catalog, CSV export
│   ├── native.ts     # Tauri IPC bridge (gracefully absent in browser mode)
│   └── style.css     # Single stylesheet, RTL-first layout
├── core/             # Rust library crate (mar2i-core)
│   └── src/
│       ├── discovery.rs   # Safe directory walk, symlink rejection, path validation
│       ├── media.rs       # ffprobe inspection, rotation, VFR detection
│       ├── policy.rs      # Codec classification → encoding decision
│       ├── runner.rs      # Single-file encode/copy, SSIM gate, SHA-256 verify
│       ├── queue.rs       # SQLite-backed job queue (rusqlite, WAL)
│       ├── worker.rs      # Concurrent batch orchestrator, pause/cancel control
│       └── report.rs      # CSV + HTML summary generation
└── src-tauri/        # Tauri shell — IPC command handlers, ffmpeg/ffprobe discovery
```

## Screens

| Screen | Purpose |
|---|---|
| New batch | Source/output folder selection, mode and codec choice |
| Options | Codec detail cards with honest trade-off descriptions |
| Review | File list analysis before processing starts |
| Progress | Live per-file status with pause/cancel |
| Results | Verified savings summary, CSV/HTML export |
| Settings | Resource profile selector |

## Processing Modes

**Publish** — Re-encodes eligible sources to H.264/MP4 with SSIM ≥ 0.965 gate. HDR, subtitle streams, and non-AAC audio are skipped with explicit reasons. Already-efficient H.264 and modern codecs (AV1, HEVC, VP9) are byte-copied without re-encoding.

**Archive** — Uncompressed raw video is encoded to FFV1/MKV with decoded-frame hash verification (identical frames required). All other sources are byte-copied and SHA-256 verified.

Both modes refuse to overwrite existing output files, verify source integrity before publishing, and reject zero-byte or larger-than-source outputs.

## Development

### Prerequisites

- [Bun](https://bun.sh) ≥ 1.1
- [Rust](https://rustup.rs) toolchain (stable, edition 2024)
- [Tauri CLI v2](https://tauri.app/start/prerequisites/)
- `ffmpeg` and `ffprobe` on `PATH` (or bundled in the app resources directory)

### Commands

```bash
# Install dependencies
bun install

# Run UI only in browser (prototype mode, no native IPC)
bun dev

# Run full desktop app (requires Tauri prerequisites)
bunx tauri dev

# Type-check
bunx tsc --noEmit

# Run TypeScript unit tests
bun test

# Build Rust core tests
cargo test --manifest-path core/Cargo.toml

# Production desktop build
bunx tauri build
```

### Testing

TypeScript pure-logic tests live in `src/data.test.ts` and run with `bun test`.  
Rust integration tests live in `core/tests/` and cover discovery, policy, queue, worker, media inspection, and report generation.

## Security properties

- Input files are never modified; output is written to a separate directory.
- Symlinks are rejected at discovery and inspection time.
- Path traversal is validated before any file operation.
- Output directory path is canonicalized and checked against the declared root before each write.
- Output files are published via atomic hard-link (no silent overwrite).
- Sources are fingerprinted before encode and re-checked before publish; a changed source aborts the job.

## License

Private — not for redistribution.
