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

export function nativeReportContent(format: 'csv' | 'html'): Promise<string> {
  return invoke<string>('report_content', { format })
}
