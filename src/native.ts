import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { getSampleProbeData } from './data'

export const isDesktop = '__TAURI_INTERNALS__' in window

export type ScanResult = {
  total: number
  totalBytes: number
  supported: number
  skipped: number
  details: string[]
}

export type JobStatus = {
  relative: string
  status: 'discovered' | 'analyzed' | 'queued' | 'processing' | 'verifying' | 'completed' | 'skipped' | 'failed' | 'interrupted'
  inputBytes: number
  outputBytes: number | null
  output: string | null
  reason: string | null
  quality: string | null
  elapsedMs: number | null
}

export type BatchStatus = {
  running: boolean
  paused: boolean
  cancelled: boolean
  error: string | null
  files: number
  completed: number
  skipped: number
  failed: number
  queued: number
  active: number
  inputBytes: number
  outputBytes: number
  verifiedSavingsBytes: number
  jobs: JobStatus[]
}

export async function pickDirectory(): Promise<string | null> {
  if (!isDesktop) return null
  const chosen = await open({ directory: true, multiple: false, title: 'اختر المجلد' })

  if (chosen === null || Array.isArray(chosen)) return null

  return chosen
}

export async function pickVideoFile(): Promise<string | null> {
  if (!isDesktop) return null

  const chosen = await open({
    directory: false,
    multiple: false,
    title: 'اختر ملف الفيديو',
    filters: [{
      name: 'Video',
      extensions: ['mp4', 'mov', 'mkv', 'avi', 'm4v', 'webm'],
    }],
  })

  if (chosen === null || Array.isArray(chosen)) return null

  return chosen
}

export type InspectedMedia = {
  path: string
  fileSize: number
  width: number
  height: number
  fps: number
  durationSeconds: number
  videoCodec: string
  videoBitrate: number | null
  totalBitrate: number | null
  audioCodec: string | null
  audioBitrate: number | null
  targetAbrKbps: number | null
  recommendedAction: string
}

export type CompressionSnapshotReport = {
  sourcePath: string
  outputPath: string
  originalSize: number
  compressedSize: number
  sizeReductionPercent: number
  originalVideoBitrate: number | null
  compressedVideoBitrate: number | null
  originalResolution: string
  compressedResolution: string
  originalFps: number
  compressedFps: number
  videoCodec: string
  audioCodec: string
  audioAction: string
  container: string
  faststart: boolean
  ssim: number | null
  psnr: number | null
  elapsedSeconds: number
}

export function inspectMediaFile(path: string): Promise<InspectedMedia> {
  return invoke<InspectedMedia>('inspect_media_file', { path })
}

export type SingleVideoOptions = {
  targetRatio?: number | null
  trimStart?: number | null
  trimEnd?: number | null
  aspectPreset?: string | null
  rotationDeg?: number | null
  flipHorizontal?: boolean | null
  hardwareAcceleration?: boolean | null
}

export function compressSingleVideo(
  sourcePath: string,
  outputDir?: string | null,
  options?: SingleVideoOptions
): Promise<CompressionSnapshotReport> {
  return invoke<CompressionSnapshotReport>('compress_single_video', {
    sourcePath,
    outputDir: outputDir ?? undefined,
    targetRatio: options?.targetRatio ?? undefined,
    trimStart: options?.trimStart ?? undefined,
    trimEnd: options?.trimEnd ?? undefined,
    aspectPreset: options?.aspectPreset ?? undefined,
    rotationDeg: options?.rotationDeg ?? undefined,
    flipHorizontal: options?.flipHorizontal ?? undefined,
    hardwareAcceleration: options?.hardwareAcceleration ?? undefined,
  })
}

export function analyzeDirectory(input: string, output: string, mode: string, codec?: string): Promise<ScanResult> {
  return invoke<ScanResult>('analyze_directory', { input, output, mode, codec })
}

export function startNativeBatch(
  input: string,
  output: string,
  mode: string,
  profile: string,
  codec: string,
  hardwareAcceleration?: boolean
): Promise<void> {
  return invoke<void>('start_batch', {
    input,
    output,
    mode,
    profile,
    codec,
    hardwareAcceleration,
  })
}

export function nativeStatus(): Promise<BatchStatus> {
  return invoke<BatchStatus>('batch_status')
}

export function nativeControl(action: 'pause' | 'resume' | 'cancel'): Promise<void> {
  return invoke<void>('control_batch', { action })
}

export function openNativeResults(path?: string): Promise<void> {
  return invoke<void>('open_output_directory', { path })
}

export async function pickSavePath(defaultName: string, extensions: string[]): Promise<string | null> {
  if (!isDesktop) return null

  const chosen = await save({
    defaultPath: defaultName,
    filters: [{
      name: 'Report',
      extensions,
    }],
  })

  if (chosen === null || Array.isArray(chosen)) return null

  return chosen
}

export function saveReportFile(format: string, destination: string): Promise<string> {
  return invoke<string>('save_report_file', { format, destination })
}

export type HardwareEncoder = {
  codec: string
  encoder: string
  api: 'video_toolbox' | 'nvenc' | 'qsv' | 'amf' | 'vaapi'
  verified: boolean
}

export type HardwareReport = {
  availableEncoders: HardwareEncoder[]
  recommendedH264: string | null
  recommendedHevc: string | null
}

export type MinistryProfile = {
  id: string
  nameAr: string
  nameEn: string
  descriptionAr: string
  descriptionEn: string
  mode: string
  codec: string
  container: string
  crf: number | null
  hardwareAccelerated: boolean
  verification: string
}

export type BatchSavingsEstimate = {
  originalBytes: number
  estimatedBytes: number
  savingsBytes: number
  savingsPercent: number
  filesCount: number
}

export type MediaFingerprint = {
  hash: string
  sizeBytes: number
}

export function detectHardwareEncoders(): Promise<HardwareReport> {
  if (!isDesktop) {
    return Promise.resolve({
      availableEncoders: [],
      recommendedH264: null,
      recommendedHevc: null,
    })
  }

  return invoke<HardwareReport>('detect_hardware_encoders')
}

export function getMinistryProfiles(): Promise<MinistryProfile[]> {
  if (!isDesktop) {
    return Promise.resolve([])
  }

  return invoke<MinistryProfile[]>('get_ministry_profiles')
}

export function estimateBatchSavings(
  input: string,
  mode: string,
  codec: string,
  crf?: number
): Promise<BatchSavingsEstimate> {
  if (!isDesktop) {
    return Promise.resolve({
      originalBytes: 0,
      estimatedBytes: 0,
      savingsBytes: 0,
      savingsPercent: 0,
      filesCount: 0,
    })
  }

  return invoke<BatchSavingsEstimate>('estimate_batch_savings', { input, mode, codec, crf })
}

export function fingerprintMedia(path: string): Promise<MediaFingerprint> {
  if (!isDesktop) {
    return Promise.resolve({ hash: '', sizeBytes: 0 })
  }

  return invoke<MediaFingerprint>('fingerprint_media', { path })
}

export function nativeReportContent(format: 'csv' | 'html'): Promise<string> {
  return invoke<string>('report_content', { format })
}

export type MediaProbeData = {
  fileName: string
  fileSize: number
  durationSeconds: number
  formatName: string
  overallBitrateKbps: number
  videoCodec: string
  videoProfile: string
  resolution: string
  fps: number
  colorSpace: string
  pixelFormat: string
  audioCodec: string
  audioChannels: number
  audioSampleRateHz: number
  audioBitrateKbps: number
  sparseSha256: string
  cameraModel?: string
  preservationClass: 'Master Permanent Archive' | 'Standard Media' | 'Raw Mezzanine'
}

export type FramePairBase64 = {
  originalBase64: string
  compressedBase64: string
  ssim: number
}

export function probeForensicMedia(path: string): Promise<MediaProbeData> {
  if (!isDesktop) {
    return Promise.resolve(getSampleProbeData(path))
  }

  return invoke<MediaProbeData>('probe_forensic_media', { path })
}

export function extractAudioPeaks(path: string, buckets?: number): Promise<number[]> {
  if (!isDesktop) {
    return Promise.resolve([])
  }

  return invoke<number[]>('extract_audio_peaks', { path, buckets })
}

export function extractComparisonFrames(
  sourcePath: string,
  outputPath?: string,
  timestampSec?: number
): Promise<FramePairBase64> {
  if (!isDesktop) {
    return Promise.resolve({
      originalBase64: '',
      compressedBase64: '',
      ssim: 0.985,
    })
  }

  return invoke<FramePairBase64>('extract_comparison_frames', {
    sourcePath,
    outputPath,
    timestampSec,
  })
}
