import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

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

export function batchProgressLabel(batch: BatchStatus | null): string {
  if (!batch) return 'لم تبدأ المعالجة بعد'
  if (batch.error) return 'تعذرت متابعة المعالجة'
  if (batch.cancelled) return 'أُلغيت الدفعة'
  if (batch.paused) return 'المعالجة متوقفة مؤقتًا'
  if (batch.running) return 'النتائج تتحدث مباشرة'
  if (batch.files === 0) return 'لم تبدأ المعالجة بعد'
  if (batch.queued > 0 || batch.active > 0) return 'الدفعة لم تكتمل بعد'
  if (batch.failed > 0) return 'اكتملت المعالجة مع ملفات تحتاج مراجعة'
  return 'اكتملت المعالجة'
}

export async function pickDirectory(): Promise<string | null> {
  if (!isDesktop) return null
  const chosen = await open({ directory: true, multiple: false, title: 'اختر المجلد' })
  return typeof chosen === 'string' ? chosen : null
}

export function analyzeDirectory(input: string, output: string, mode: string): Promise<ScanResult> {
  return invoke<ScanResult>('analyze_directory', { input, output, mode })
}

export function startNativeBatch(input: string, output: string, mode: string, profile: string, codec: string): Promise<void> {
  return invoke<void>('start_batch', { input, output, mode, profile, codec })
}

export function nativeStatus(): Promise<BatchStatus> {
  return invoke<BatchStatus>('batch_status')
}

export function nativeControl(action: 'pause' | 'resume' | 'cancel'): Promise<void> {
  return invoke<void>('control_batch', { action })
}

export function openNativeResults(): Promise<void> {
  return invoke<void>('open_output_directory')
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
