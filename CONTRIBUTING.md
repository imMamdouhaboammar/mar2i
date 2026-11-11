## Reporting issues

Open an issue describing the observed behavior, the expected behavior, and the reproduction steps. Include the OS version, `ffmpeg -version` output, and whether the failure occurs in browser prototype mode or the full desktop build.

## Pull requests

1. **One concern per PR.** Mix of features, refactors, and test changes in one diff is a review burden.
2. **Gate 0 before editing.** Read `PRODUCT.md`, `README.md`, and the relevant source modules before writing code.
3. **Tests required for logic changes.** New behavior in `core/` needs a corresponding test in `core/tests/`. New pure functions in `src/data.ts` need a test in `src/data.test.ts`.
4. **No FFmpeg call mocking in Rust integration tests.** Tests that need an encoding result must either use real `ffmpeg`/`ffprobe` on a minimal fixture or test at a boundary that does not invoke the binary.
5. **Source files are immutable.** The core must never modify input files. Any path that could write to the source directory is a blocking defect.
6. **Arabic copy is not placeholder text.** UI strings are production-grade. Do not replace them with English or Lorem Ipsum unless the feature itself is changing the locale.

## Code conventions

- **Rust:** `cargo clippy -- -D warnings` must be clean. No `unwrap()` on paths that reach user data; use `?` and `anyhow::Context`.
- **TypeScript:** `tsc --noEmit` clean. `noUnusedLocals` and `noUnusedParameters` are enforced.
- **No new runtime dependencies** without documented rationale (license, maintenance, bundle impact).
- **Comments explain why, not what.** Syntax-narration comments (`// return result`) are removed on review.
