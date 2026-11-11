import './test-setup'
import { describe, expect, it } from 'bun:test'
import { batchProgressLabel, type BatchStatus } from './native'

const idleBatch: BatchStatus = {
  running: false,
  paused: false,
  cancelled: false,
  error: null,
  files: 3,
  completed: 0,
  skipped: 0,
  failed: 0,
  queued: 3,
  active: 0,
  inputBytes: 0,
  outputBytes: 0,
  verifiedSavingsBytes: 0,
  jobs: [],
}

describe('native batch progress labels', () => {
  it('does not claim completion when no batch exists or queued work remains', () => {
    expect(batchProgressLabel(null)).toBe('لم تبدأ المعالجة بعد')
    expect(batchProgressLabel({ ...idleBatch, files: 0, queued: 0 })).toBe('لم تبدأ المعالجة بعد')
    expect(batchProgressLabel(idleBatch)).toBe('الدفعة لم تكتمل بعد')
  })

  it('distinguishes running, paused, cancelled, and error states', () => {
    expect(batchProgressLabel({ ...idleBatch, running: true })).toBe('النتائج تتحدث مباشرة')
    expect(batchProgressLabel({ ...idleBatch, running: true, paused: true })).toBe('المعالجة متوقفة مؤقتًا')
    expect(batchProgressLabel({ ...idleBatch, cancelled: true })).toBe('أُلغيت الدفعة')
    expect(batchProgressLabel({ ...idleBatch, error: 'ffmpeg error' })).toBe('تعذرت متابعة المعالجة')
  })

  it('discloses failed files even after all jobs settle', () => {
    expect(batchProgressLabel({ ...idleBatch, queued: 0, failed: 1, completed: 2 })).toBe('اكتملت المعالجة مع ملفات تحتاج مراجعة')
    expect(batchProgressLabel({ ...idleBatch, queued: 0, completed: 3 })).toBe('اكتملت المعالجة')
  })
})
