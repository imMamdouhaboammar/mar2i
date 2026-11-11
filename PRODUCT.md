# مرئي

<!-- impeccable:product-schema 1 -->

## Platform
Offline desktop application in development (macOS and Windows targets)

## Stack
Tauri 2, Rust processing core and SQLite jobs, TypeScript/Vite UI with Bun

## Users
Employees handling large collections of video footage for the Saudi Ministry of Hajj and Umrah

## Product Purpose
Make bulk video compression and archiving easy to configure, monitor, and report on, with all media processing intended to remain local in the eventual desktop application

## Capabilities and Constraints
- Arabic RTL first, English support planned in the later implementation
- Every codec choice has a plain-language description and trade-offs
- The preview clearly distinguishes between interactive demonstration and native local execution
- Original input files are never changed by the application (read-only guarantee)
- Reference screens: new batch, codec choices & estimation, visual A/B review, processing progress with KeepAwake, and verified results
- Hardware acceleration auto-detection with strict policy separation: GPU for Publishing, mathematically verified CPU for Archival
- Zero-leak RAII OS Sleep Inhibitor (`KeepAwake`) preventing computer sleep during multi-hour batch runs
- Stratified multi-sample empirical size estimation predicting accurate GB savings before transcode begins
- Experimental sparse SHA-256 similarity hint (size + 1MiB head + 1MiB tail). Not a complete integrity proof; batch deduplication does not use this registry
- Experimental, opt-in MCP stdio tool access; authentication and path-grant boundaries are not implemented
- Accessible RTL A/B comparison prototype using sample imagery; not connected to real native files or measured SSIM yet

## Evidence on Hand
Four generated UI reference images from the product discussion, with real implementation requirements still pending for FFmpeg processing

## Product Principles
- Keep the default path short while offering explicit detailed choices
- Explain trade-offs honestly without unsupported compression promises
- Treat source media and user trust as more important than cosmetic progress
