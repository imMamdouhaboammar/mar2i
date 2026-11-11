# مرئي

<!-- impeccable:product-schema 1 -->

## Platform
web

## Stack
HTML/CSS/TypeScript with Bun and Vite, as an interactive desktop-sized prototype

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
- Sparse SHA-256 media fingerprinting (size + 1MB head + 1MB tail) preventing redundant re-compression
- Native Model Context Protocol (MCP) JSON-RPC 2.0 stdio server enabling automated AI agent orchestration
- Accessible RTL-native Visual A/B Frame Comparator with real-time SSIM quality index verification

## Evidence on Hand
Four generated UI reference images from the product discussion, with real implementation requirements still pending for FFmpeg processing

## Product Principles
- Keep the default path short while offering explicit detailed choices
- Explain trade-offs honestly without unsupported compression promises
- Treat source media and user trust as more important than cosmetic progress
