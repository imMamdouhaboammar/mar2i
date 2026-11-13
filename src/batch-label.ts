import type { BatchStatus } from './native'

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
