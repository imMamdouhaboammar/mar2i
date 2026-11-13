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
  ssim?: number
}

export type MinistryPreset = {
  id: string
  name: string
  nameEn: string
  codec: CodecId
  mode: Mode
  crf: number
  hardwareAllowed: boolean
  description: string
  badge: string
}

export const ministryPresets: MinistryPreset[] = [
  {
    id: 'master_archive',
    name: 'الأرشفة الوثائقية غير المنقوصة',
    nameEn: 'Master Archival (FFV1)',
    codec: 'ffv1',
    mode: 'archive',
    crf: 0,
    hardwareAllowed: false,
    description: 'حفظ فوتوغرافي ومرئي غير منقوص بدون أي فقد مع فحص تطابق البصمات الإطارية framemd5',
    badge: 'معتمد للأرشفة الوطنية',
  },
  {
    id: 'broadcast_hd',
    name: 'البث التلفزيوني عالي الدقة (4K/1080p)',
    nameEn: 'Broadcasting Master (HEVC)',
    codec: 'hevc',
    mode: 'publish',
    crf: 22,
    hardwareAllowed: true,
    description: 'ترميز عالي الكفاءة يدعم تسريع العتاد GPU مخصص للشاشات والبث التلفزيوني الرسمي',
    badge: 'عالي الدقة + تسريع عتادي',
  },
  {
    id: 'press_universal',
    name: 'النشر الإعلامي وشبكات التواصل',
    nameEn: 'Universal Press & Social (H.264)',
    codec: 'h264',
    mode: 'publish',
    crf: 24,
    hardwareAllowed: true,
    description: 'توافق شامل 100% مع كافة الهواتف والمتصفحات والمراكز الصحفية للمشاعر المقدسة',
    badge: 'الأوسع انتشارًا وتوافقًا',
  },
  {
    id: 'field_mobile',
    name: 'التداول الميداني السريع (شبكات المشاعر)',
    nameEn: 'Field Mobile Delivery (AV1)',
    codec: 'av1',
    mode: 'publish',
    crf: 28,
    hardwareAllowed: false,
    description: 'أقصى ضغط ممكن لتمكين المراسلين من إرسال الفيديو عبر شبكات الجوال في منى وعرفات',
    badge: 'توفير فائق للمساحة',
  },
]

export const codecs: Codec[] = [
  { id: 'auto', name: 'تلقائي', size: 'متوازن', speed: 'حسب الملف', compatibility: 'حسب الهدف', description: 'نفحص خصائص كل فيديو ونختار إعدادًا مناسبًا للهدف المحدد', caveat: 'قد نترك بعض الملفات كما هي إذا كانت إعادة الضغط غير مفيدة', recommendation: 'الاختيار الأنسب عند معالجة مجموعة كبيرة من الملفات المختلفة', bestFor: 'both' },
  { id: 'h264', name: 'H.264', size: 'أكبر نسبيًا', speed: 'سريع', compatibility: 'الأوسع', description: 'اختيار مناسب للتداول والتشغيل على نطاق واسع من الأجهزة والمتصفحات', caveat: 'قد ينتج ملفًا أكبر من الترميزات الأحدث عند جودة مماثلة', recommendation: 'مناسب عندما تكون سهولة تشغيل الفيديو أهم من أصغر حجم ممكن', bestFor: 'publish' },
  { id: 'hevc', name: 'H.265 / HEVC', size: 'أصغر غالبًا', speed: 'متوسط', compatibility: 'جيد', description: 'يوفر عادةً ضغطًا أعلى من H.264، مع دعم تسريع البطاقة الرسومية', caveat: 'تختلف قابلية التشغيل في المتصفحات والأجهزة القديمة', recommendation: 'مناسب لملفات 4K وللأجهزة التي تدعم HEVC', bestFor: 'publish' },
  { id: 'av1', name: 'AV1', size: 'صغير غالبًا', speed: 'أبطأ', compatibility: 'متفاوت', description: 'يساعد على الوصول إلى حجم أقل عند جودة بصرية متقاربة', caveat: 'الضغط قد يستغرق وقتًا أطول، وبعض الأجهزة القديمة لا تشغله', recommendation: 'مناسب عندما تدعم المنصة والأجهزة هذا الترميز', bestFor: 'publish' },
  { id: 'vp9', name: 'VP9', size: 'صغير', speed: 'متوسط', compatibility: 'واسع للويب', description: 'خيار مناسب للنشر عبر الويب في حاويات متوافقة مثل WebM', caveat: 'ليس الخيار المثالي لكل تطبيقات المشاركة ومشغلات الفيديو', recommendation: 'مناسب إذا كانت منصة النشر تدعم WebM', bestFor: 'publish' },
  { id: 'ffv1', name: 'FFV1', size: 'كبير نسبيًا', speed: 'متوسط', compatibility: 'أرشفة', description: 'يحفظ بيانات الصورة المفككة دون فقد رياضي عند عدم تغيير خصائصها', caveat: 'غير مناسب عادةً للنشر المباشر وقد يزيد حجم الملفات المضغوطة بالفعل', recommendation: 'مناسب للأرشفة عندما تكون المطابقة الدقيقة شرطًا أساسيًا', bestFor: 'archive' },
]

export const sampleFiles: FileRow[] = [
  { name: 'دليل المناسك.mp4', before: 10.2, after: 2.4, status: 'complete', codec: 'H.265', ssim: 0.988 },
  { name: 'رسالة ترحيبية.mov', before: 189.7, after: 64.1, status: 'complete', codec: 'AV1', ssim: 0.982 },
  { name: 'إرشادات السعي.mp4', before: 75, after: 18.9, status: 'complete', codec: 'H.264', ssim: 0.985 },
  { name: 'تسجيل_14.mov', before: 4.3, after: 4.3, status: 'review', codec: 'H.264', ssim: 0.991 },
  { name: 'خدمات الحجاج.mov', before: 31.6, after: 14.8, status: 'complete', codec: 'HEVC', ssim: 0.989 },
  { name: 'رحلة المعتمر_04.mp4', before: 2.8, after: 2.8, status: 'unchanged', codec: 'H.264', ssim: 1.000 },
]

export const getCodec = (id: CodecId): Codec => codecs.find((codec) => codec.id === id) ?? codecs[0]

export function recommendedCodec(mode: Mode): CodecId {
  return mode === 'archive' ? 'ffv1' : 'h264'
}

export function savingsPercent(before: number, after: number): number {
  if (before <= 0 || !Number.isFinite(before) || !Number.isFinite(after)) return 0

  return Math.round(((before - after) / before) * 100)
}

export interface PageWindow {
  page: number
  pages: number
  start: number
  end: number
}

export interface CodecSavingsProjection {
  projectedGb: number
  savedGb: number
  percent: number
}

export function progressPageWindow(total: number, requestedPage: number): PageWindow {
  const pages = Math.max(1, Math.ceil(total / 30))
  const page = Math.min(pages - 1, Math.max(0, Math.trunc(requestedPage)))

  return { page, pages, start: page * 30, end: Math.min(total, (page + 1) * 30) }
}

export function projectCodecSavings(totalGb: number, codec: CodecId, mode: Mode): CodecSavingsProjection {
  if (totalGb <= 0) return { projectedGb: 0, savedGb: 0, percent: 0 }

  if (mode === 'archive') {
    // Archival FFV1 typical ~45% reduction from uncompressed masters
    const projected = Math.round(totalGb * 0.55 * 10) / 10
    const saved = Math.round((totalGb - projected) * 10) / 10

    return { projectedGb: projected, savedGb: saved, percent: 45 }
  }

  const factor = {
    auto: 0.40,
    av1: 0.30,
    hevc: 0.38,
    vp9: 0.45,
    h264: 0.55,
    ffv1: 0.95,
  }[codec] ?? 0.50

  const projectedGb = Math.round(totalGb * factor * 10) / 10
  const savedGb = Math.round((totalGb - projectedGb) * 10) / 10
  const percent = Math.round(((totalGb - projectedGb) / totalGb) * 100)

  return { projectedGb, savedGb, percent }
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


export function getSampleProbeData(fileName: string) {
  const isMov = fileName.toLowerCase().endsWith('.mov')

  return {
    fileName,
    fileSize: isMov ? 742 * 1024 * 1024 : 164 * 1024 * 1024,
    durationSeconds: isMov ? 342 : 128,
    formatName: isMov ? 'mov,mp4,m4a,3gp,3g2,mj2' : 'mov,mp4,m4a',
    overallBitrateKbps: isMov ? 18200 : 10700,
    videoCodec: isMov ? 'Apple ProRes 422 HQ (apch)' : 'H.264 / AVC / MPEG-4 AVC',
    videoProfile: isMov ? 'High 4:2:2 Profile' : 'High Profile (L4.2)',
    resolution: isMov ? '3840x2160 (4K UHD)' : '1920x1080 (FHD)',
    fps: 59.94,
    colorSpace: isMov ? 'bt709 (10-bit SDR Mezzanine)' : 'bt709 (8-bit SDR)',
    pixelFormat: isMov ? 'yuv422p10le' : 'yuv420p',
    audioCodec: isMov ? 'pcm_s24le (Linear PCM)' : 'aac (LC)',
    audioChannels: 2,
    audioSampleRateHz: 48000,
    audioBitrateKbps: isMov ? 2304 : 320,
    sparseSha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
    cameraModel: 'Sony FX6 Cinema Line (المسجد الحرام - الصفا)',
    preservationClass: isMov ? ('Master Permanent Archive' as const) : ('Standard Media' as const),
  }
}
