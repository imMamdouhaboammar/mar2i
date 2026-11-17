/**
 * Mar2i Offline Multimedia Packaging Tool
 *
 * Prepares verified, standalone FFmpeg and FFprobe binaries for production desktop builds.
 * Enforces cryptographic SHA-256 checksum verification before staging into src-tauri/bin/.
 */

import { createHash } from "node:crypto";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";

interface BinarySpec {
  name: string;
  expectedSha256: string;
  sourceUrl?: string;
  isArchive?: boolean;
  archivePath?: string;
  isGzip?: boolean;
}

interface TargetConfig {
  id: string;
  platform: string;
  architecture: string;
  binaries: BinarySpec[];
}

const REPO_ROOT = resolve(import.meta.dirname, "..");
const BIN_DIR = join(REPO_ROOT, "src-tauri", "bin");
const CACHE_DIR = join(REPO_ROOT, ".cache", "media-binaries");

const TARGET_CATALOG = {
  "macos-arm64": {
    id: "macos-arm64",
    platform: "darwin",
    architecture: "arm64",
    binaries: [
      {
        name: "ffmpeg",
        expectedSha256:
          "84a82ad0b053e2c8051d42d4e7f96b1a64ce2f6f3be07619eebf5eee10c6390c",
        sourceUrl:
          "https://github.com/kiimelon/FFmpeg-arm-silicon/releases/download/0.5.0/FFmpeg-arm-silicon-Tools-20260424.zip",
        isArchive: true,
        archivePath: "Tools/ffmpeg",
      },
      {
        name: "ffprobe",
        expectedSha256:
          "8733960013e6aed00a513e9cecb5026aadc88c6e5d79ae6191fd9ea892a16e41",
        sourceUrl:
          "https://github.com/kiimelon/FFmpeg-arm-silicon/releases/download/0.5.0/FFmpeg-arm-silicon-Tools-20260424.zip",
        isArchive: true,
        archivePath: "Tools/ffprobe",
      },
    ],
  },
  "windows-x64": {
    id: "windows-x64",
    platform: "windows",
    architecture: "x64",
    binaries: [
      {
        name: "ffmpeg.exe",
        expectedSha256:
          "04e1307997530f9cf2fe35cba2ca7e8875ca91da02f89d6c7243df819c94ad00",
        sourceUrl:
          "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-win32-x64.gz",
        isGzip: true,
      },
      {
        name: "ffprobe.exe",
        expectedSha256:
          "4303ec85855340689b1f8aa5d9c1dc06ef3e3090682de3034edc3fca2b0798d5",
        sourceUrl:
          "https://raw.githubusercontent.com/joshwnj/ffprobe-static/master/bin/win32/x64/ffprobe.exe",
      },
    ],
  },
} satisfies Record<string, TargetConfig>;

type TargetKey = keyof typeof TARGET_CATALOG;

function computeSha256(buffer: Buffer): string {
  return createHash("sha256").update(buffer).digest("hex");
}

async function fetchOrUseLocal(
  url: string,
  localCachePath: string
): Promise<Buffer> {
  if (existsSync(localCachePath)) {
    return readFileSync(localCachePath);
  }

  mkdirSync(CACHE_DIR, { recursive: true });
  console.log(`Downloading: ${url}`);

  const response = await fetch(url, { redirect: "follow" });

  if (!response.ok) {
    throw new Error(`Failed to fetch ${url} (status ${response.status})`);
  }

  const arrayBuffer = await response.arrayBuffer();
  const buffer = Buffer.from(arrayBuffer);

  writeFileSync(localCachePath, buffer);

  return buffer;
}

async function resolveBinary(
  spec: BinarySpec,
  targetId: string
): Promise<Buffer> {
  if (targetId === "macos-arm64") {
    const tmpArmPath = join("/tmp/ffmpeg-arm", spec.archivePath || "");

    if (existsSync(tmpArmPath)) {
      const buffer = readFileSync(tmpArmPath);
      const hash = computeSha256(buffer);

      if (hash === spec.expectedSha256) {
        return buffer;
      }
    }
  }

  if (targetId === "windows-x64") {
    if (spec.name === "ffmpeg.exe" && existsSync("/tmp/ffmpeg.exe")) {
      const buffer = readFileSync("/tmp/ffmpeg.exe");
      const hash = computeSha256(buffer);

      if (hash === spec.expectedSha256) {
        return buffer;
      }
    }

    const tmpNodeProbe =
      "/tmp/mar2i-test-fetch/node_modules/.bun/ffprobe-static@3.1.0/node_modules/ffprobe-static/bin/win32/x64/ffprobe.exe";

    if (spec.name === "ffprobe.exe" && existsSync(tmpNodeProbe)) {
      const buffer = readFileSync(tmpNodeProbe);
      const hash = computeSha256(buffer);

      if (hash === spec.expectedSha256) {
        return buffer;
      }
    }
  }

  if (!spec.sourceUrl) {
    throw new Error(`No source URL or local candidate found for ${spec.name}`);
  }

  if (spec.isArchive) {
    const zipName = "tools-arm.zip";
    const cacheZipPath = join(CACHE_DIR, zipName);

    await fetchOrUseLocal(spec.sourceUrl, cacheZipPath);

    const proc = Bun.spawnSync([
      "unzip",
      "-p",
      cacheZipPath,
      spec.archivePath!,
    ]);

    if (proc.exitCode !== 0) {
      throw new Error(
        `Failed to extract ${spec.archivePath} from ${cacheZipPath}`
      );
    }

    return Buffer.from(proc.stdout);
  }

  if (spec.isGzip) {
    const gzCache = join(CACHE_DIR, `${spec.name}.gz`);
    const gzBuffer = await fetchOrUseLocal(spec.sourceUrl, gzCache);
    const unzipped = Bun.gunzipSync(gzBuffer);

    return Buffer.from(unzipped);
  }

  const directCache = join(CACHE_DIR, spec.name);

  return await fetchOrUseLocal(spec.sourceUrl, directCache);
}

export async function stageTarget(targetKey: TargetKey): Promise<void> {
  const target: TargetConfig = TARGET_CATALOG[targetKey];

  if (!target) {
    throw new Error(
      `Unknown target: ${targetKey}. Supported: ${Object.keys(TARGET_CATALOG).join(", ")}`
    );
  }

  console.log(`\n========================================`);
  console.log(`Staging offline binaries for target: ${target.id}`);
  console.log(`========================================`);

  mkdirSync(BIN_DIR, { recursive: true });

  for (const file of readdirSync(BIN_DIR)) {
    if (file !== ".gitkeep") {
      unlinkSync(join(BIN_DIR, file));
    }
  }

  const manifestEntries = [];

  for (const spec of target.binaries) {
    console.log(`Resolving ${spec.name}...`);

    const buffer = await resolveBinary(spec, target.id);
    const actualHash = computeSha256(buffer);

    if (actualHash !== spec.expectedSha256) {
      throw new Error(
        `SHA-256 verification failed for ${spec.name}!\nExpected: ${spec.expectedSha256}\nActual:   ${actualHash}`
      );
    }

    const destPath = join(BIN_DIR, spec.name);

    writeFileSync(destPath, buffer);

    if (!spec.name.endsWith(".exe")) {
      chmodSync(destPath, 0o755);
    }

    console.log(
      `✓ Verified ${spec.name} (${(buffer.length / 1024 / 1024).toFixed(2)} MB) - SHA256: ${actualHash.slice(0, 16)}...`
    );

    manifestEntries.push({
      name: spec.name,
      sha256: actualHash,
      bytes: buffer.length,
      executable: true,
    });
  }

  const manifest = {
    target: target.id,
    platform: target.platform,
    architecture: target.architecture,
    stagedAt: new Date().toISOString(),
    binaries: manifestEntries,
  };

  writeFileSync(
    join(BIN_DIR, "MANIFEST.json"),
    JSON.stringify(manifest, null, 2) + "\n"
  );

  console.log(`✓ Generated: src-tauri/bin/MANIFEST.json`);
}

function verifyStaging(): void {
  const manifestPath = join(BIN_DIR, "MANIFEST.json");

  if (!existsSync(manifestPath)) {
    console.log("No manifest found in src-tauri/bin/. Run stage command first.");

    return;
  }

  const manifest = JSON.parse(readFileSync(manifestPath, "utf-8"));

  console.log(
    `Current staged target: ${manifest.target} (${manifest.stagedAt})`
  );

  for (const bin of manifest.binaries) {
    const binPath = join(BIN_DIR, bin.name);

    if (!existsSync(binPath)) {
      console.error(`✗ Missing binary: ${bin.name}`);
      continue;
    }

    const content = readFileSync(binPath);
    const hash = computeSha256(content);

    if (hash === bin.sha256) {
      console.log(
        `✓ ${bin.name}: verified (${bin.bytes} bytes, hash: ${hash.slice(0, 16)}...)`
      );
    } else {
      console.error(
        `✗ ${bin.name}: hash mismatch! Expected ${bin.sha256}, got ${hash}`
      );
    }
  }
}

const args = process.argv.slice(2);

const targetArg =
  args.find((a) => a.startsWith("--target="))?.split("=")[1] || args[0];

if (targetArg === "verify" || args.includes("--verify")) {
  verifyStaging();
} else if (targetArg === "clean" || args.includes("--clean")) {
  for (const file of readdirSync(BIN_DIR)) {
    if (file !== ".gitkeep") {
      unlinkSync(join(BIN_DIR, file));
    }
  }

  console.log("Cleaned src-tauri/bin/");
} else if (targetArg && targetArg in TARGET_CATALOG) {
  // SAFETY: verified by targetArg in TARGET_CATALOG membership check above.
  await stageTarget(targetArg as TargetKey);
  verifyStaging();
} else {
  console.log(
    `Usage: bun run tools/package-offline.ts --target=<macos-arm64|windows-x64|verify|clean>`
  );
  console.log(`Available targets: ${Object.keys(TARGET_CATALOG).join(", ")}`);
}
