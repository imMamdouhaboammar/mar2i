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
- The preview must not imply that video encoding or archival verification is implemented
- Original input files are never changed by the prototype
- The four reference screens are new batch, codec choices, processing progress, and results, with an additional review step

## Evidence on Hand
Four generated UI reference images from the product discussion, with real implementation requirements still pending for FFmpeg processing

## Product Principles
- Keep the default path short while offering explicit detailed choices
- Explain trade-offs honestly without unsupported compression promises
- Treat source media and user trust as more important than cosmetic progress
