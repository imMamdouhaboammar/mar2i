export type Screen = 'new' | 'options' | 'review' | 'progress' | 'results' | 'settings'
export type Mode = 'publish' | 'archive'
export type CodecId = 'auto' | 'h264' | 'hevc' | 'av1' | 'vp9' | 'ffv1'
export type Resource = 'low' | 'balanced' | 'maximum'

export type Codec = {
  id: CodecId
  name: string
  size: string
  speed: string
  compatibility: string
  description: string
  caveat: string
  recommendation: string
  bestFor: Mode | 'both'
}

export type FileRow = {
  name: string
  before: number
  after: number
  status: 'complete' | 'unchanged' | 'review'
  codec: string
}

export const codecs: Codec[] = [
  { id: 'auto', name: 'تلقائي', size: 'متوازن', speed: 'حسب الملف', compatibility: 'حسب الهدف', description: 'نفحص خصائص كل فيديو ونختار إعدادًا مناسبًا للهدف المحدد', caveat: 'قد نترك بعض الملفات كما هي إذا كانت إعادة الضغط غير مفيدة', recommendation: 'الاختيار الأنسب عند معالجة مجموعة كبيرة من الملفات المختلفة', bestFor: 'both' },
  { id: 'h264', name: 'H.264', size: 'أكبر نسبيًا', speed: 'سريع', compatibility: 'الأوسع', description: 'اختيار مناسب للتداول والتشغيل على نطاق واسع من الأجهزة والمتصفحات', caveat: 'قد ينتج ملفًا أكبر من الترميزات الأحدث عند جودة مماثلة', recommendation: 'مناسب عندما تكون سهولة تشغيل الفيديو أهم من أصغر حجم ممكن', bestFor: 'publish' },
  { id: 'hevc', name: 'H.265 / HEVC', size: 'أصغر غالبًا', speed: 'متوسط', compatibility: 'جيد', description: 'يوفر عادةً ضغطًا أعلى من H.264، مع دعم جيد على الأجهزة الحديثة', caveat: 'تختلف قابلية التشغيل في المتصفحات والأجهزة القديمة', recommendation: 'مناسب لملفات 4K وللأجهزة التي تدعم HEVC', bestFor: 'publish' },
  { id: 'av1', name: 'AV1', size: 'صغير غالبًا', speed: 'أبطأ', compatibility: 'متفاوت', description: 'يساعد على الوصول إلى حجم أقل عند جودة بصرية متقاربة', caveat: 'الضغط قد يستغرق وقتًا أطول، وبعض الأجهزة القديمة لا تشغله', recommendation: 'مناسب عندما تدعم المنصة والأجهزة هذا الترميز', bestFor: 'publish' },
  { id: 'vp9', name: 'VP9', size: 'صغير', speed: 'متوسط', compatibility: 'واسع للويب', description: 'خيار مناسب للنشر عبر الويب في حاويات متوافقة مثل WebM', caveat: 'ليس الخيار المثالي لكل تطبيقات المشاركة ومشغلات الفيديو', recommendation: 'مناسب إذا كانت منصة النشر تدعم WebM', bestFor: 'publish' },
  { id: 'ffv1', name: 'FFV1', size: 'كبير نسبيًا', speed: 'متوسط', compatibility: 'أرشفة', description: 'يحفظ بيانات الصورة المفككة دون فقد رياضي عند عدم تغيير خصائصها', caveat: 'غير مناسب عادةً للنشر المباشر وقد يزيد حجم الملفات المضغوطة بالفعل', recommendation: 'مناسب للأرشفة عندما تكون المطابقة الدقيقة شرطًا أساسيًا', bestFor: 'archive' },
]

export const sampleFiles: FileRow[] = [
  { name: 'دليل المناسك.mp4', before: 10.2, after: 2.4, status: 'complete', codec: 'H.265' },
  { name: 'رسالة ترحيبية.mov', before: 189.7, after: 64.1, status: 'complete', codec: 'AV1' },
  { name: 'إرشادات السعي.mp4', before: 75, after: 18.9, status: 'complete', codec: 'H.264' },
  { name: 'تسجيل_14.mov', before: 4.3, after: 4.3, status: 'review', codec: 'H.264' },
  { name: 'خدمات الحجاج.mov', before: 31.6, after: 14.8, status: 'complete', codec: 'HEVC' },
  { name: 'رحلة المعتمر_04.mp4', before: 2.8, after: 2.8, status: 'unchanged', codec: 'H.264' },
]

export const getCodec = (id: CodecId): Codec => codecs.find((codec) => codec.id === id) ?? codecs[0]

export function recommendedCodec(mode: Mode): CodecId {
  return mode === 'archive' ? 'ffv1' : 'h264'
}

export function savingsPercent(before: number, after: number): number {
  if (before <= 0 || !Number.isFinite(before) || !Number.isFinite(after)) return 0
  return Math.round(((before - after) / before) * 100)
}

export function safeCsvCell(value: string | number): string {
  const raw = String(value)
  const protectedValue = /^[\s]*[=+@\-\t\r]/.test(raw) ? `'${raw}` : raw
  return `"${protectedValue.replaceAll('"', '""')}"`
}

export function reportCsv(rows: FileRow[]): string {
  const header = ['الملف', 'الحجم قبل GB', 'الحجم بعد GB', 'التوفير %', 'الحالة', 'الترميز']
  const lines = rows.map((row) => [row.name, row.before, row.after, savingsPercent(row.before, row.after), row.status, row.codec].map(safeCsvCell).join(','))
  return '\uFEFF' + [header.map(safeCsvCell).join(','), ...lines].join('\r\n')
}

export function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]!)
}
