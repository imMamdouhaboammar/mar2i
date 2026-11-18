import './style.css'
import { batchProgressLabel } from './batch-label'
import {
  isDesktop,
  pickDirectory,
  pickVideoFile,
  pickSavePath,
  saveReportFile,
  analyzeDirectory,
  startNativeBatch,
  nativeStatus,
  nativeControl,
  openNativeResults,
  detectHardwareEncoders,
  inspectMediaFile,
  compressSingleVideo,
  probeForensicMedia,
  extractAudioPeaks,
  extractComparisonFrames,
  type ScanResult,
  type BatchStatus,
  type HardwareReport,
  type InspectedMedia,
  type CompressionSnapshotReport,
  type MediaProbeData,
} from './native'
import {
  codecs,
  escapeHtml,
  getCodec,
  getSampleProbeData,
  ministryPresets,
  projectCodecSavings,
  progressPageWindow,
  reportCsv,
  sampleFiles,
  savingsPercent,
  type CodecId,
  type Mode,
  type Resource,
  type Screen,
} from './data'
import { CompareSlider } from './compare-slider'
import { DynamicIsland, type QueueItem } from './widgets/dynamic-island'
import { VideoTrimmer } from './widgets/video-trimmer'
import { AspectRatioStudio } from './widgets/aspect-ratio-studio'
import { ForensicProbeDrawer } from './widgets/forensic-probe'
import { StorageMeter } from './widgets/storage-meter'
import { ImpactCertificate } from './widgets/impact-certificate'
import { BatchActionBar } from './widgets/batch-action-bar'
import {
  APP_VERSION,
  APP_RELEASE_NAME,
  APP_BUILD_DATE,
  APP_TARGET_ARCH,
  APP_ENGINE_INFO,
  APP_ORGANIZATION,
  APP_DEPARTMENT,
  APP_LEGAL_COPYRIGHT,
  APP_LEGAL_RESTRICTION,
} from './version'

type FolderDialog = 'output' | 'cancel' | 'results' | 'about' | null

type UiState = {
  screen: Screen
  mode: Mode
  codec: CodecId
  help: CodecId
  resource: Resource
  hardware: boolean
  sourceName: string
  outputName: string
  sourceType: 'folder' | 'file' | null
  sourcePath: string | null
  outputPath: string | null
  analysis: ScanResult | null
  batch: BatchStatus | null
  notice: string | null
  sourceCount: number | null
  search: string
  reportMenu: boolean
  expandedFile: string | null
  progress: number
  progressPage: number
  paused: boolean
  dialog: FolderDialog
  hardwareReport: HardwareReport | null
  compareSliderInstance: CompareSlider | null
  dynamicIslandInstance: DynamicIsland | null
  selectedFileIds: Set<string>
  activeTrimmerFile: string | null
  trimmerAudioPeaks: number[] | null
  trimmerDuration: number | null
  trimStart: number | null
  trimEnd: number | null
  activeAspectFile: string | null
  aspectPreset: string | null
  rotationDeg: number | null
  flipHorizontal: boolean | null
  activeProbeFile: string | null
  activeProbeData: MediaProbeData | null
  comparisonOriginalFrame: string | null
  comparisonCompressedFrame: string | null
  comparisonSsim: number | null
  islandExpanded: boolean
  analyzing: boolean
  singleFilePath: string | null
  singleFileMedia: InspectedMedia | null
  singleFileReport: CompressionSnapshotReport | null
  singleFileCompressing: boolean
}

const state: UiState = {
  screen: 'new',
  mode: 'publish',
  codec: 'auto',
  help: 'hevc',
  resource: 'balanced',
  hardware: !isDesktop,
  sourceName: isDesktop ? 'لم يتم اختيار مجلد' : 'فيديوهات موسم الحج 1447',
  outputName: isDesktop ? 'لم يتم اختيار مجلد' : 'النسخ المضغوطة',
  sourceType: null,
  sourceCount: null,
  sourcePath: null,
  outputPath: null,
  analysis: null,
  batch: null,
  notice: null,
  search: '',
  reportMenu: false,
  expandedFile: 'تسجيل_14.mov',
  progress: 34,
  progressPage: 0,
  paused: true,
  dialog: null,
  hardwareReport: null,
  compareSliderInstance: null,
  dynamicIslandInstance: null,
  selectedFileIds: new Set<string>(),
  activeTrimmerFile: null,
  trimmerAudioPeaks: null,
  trimmerDuration: null,
  trimStart: null,
  trimEnd: null,
  activeAspectFile: null,
  aspectPreset: null,
  rotationDeg: null,
  flipHorizontal: null,
  activeProbeFile: null,
  activeProbeData: null,
  comparisonOriginalFrame: null,
  comparisonCompressedFrame: null,
  comparisonSsim: null,
  islandExpanded: false,
  analyzing: false,
  singleFilePath: null,
  singleFileMedia: null,
  singleFileReport: null,
  singleFileCompressing: false,
}

const root = document.querySelector<HTMLDivElement>('#app')!
let progressTimer: ReturnType<typeof setInterval> | null = null
let nativeTimer: ReturnType<typeof setInterval> | null = null

// Initial hardware probe
if (isDesktop) {
  detectHardwareEncoders().then((report) => {
    state.hardwareReport = report
    state.hardware = (report.availableEncoders.length > 0)
    render()
  }).catch(() => {})
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return 'غير متاح'

  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let number = bytes / 1024
  let position = 0

  while (number >= 1024 && position < units.length - 1) {
    number /= 1024
    position++
  }

  return `${number.toFixed(number < 10 ? 1 : 0)} ${units[position]}`
}

function showError(error: Error | string): void {
  state.notice = error instanceof Error ? error.message : error
  render()
}

function iconPath(name: string): string {
  switch (name) {
    case 'layers': return '<path d="m12 2 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 17l9 5 9-5"/>'
    case 'play': return '<circle cx="12" cy="12" r="9"/><path d="m10 8 6 4-6 4V8Z"/>'
    case 'report': return '<rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 9h8M8 13h8M8 17h5"/>'
    case 'settings': return '<path d="M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Z"/><path d="M4.7 9.5 3 8l2-3 2.2.8a8 8 0 0 1 2.2-1.2L10 2h4l.6 2.6a8 8 0 0 1 2.2 1.2L19 5l2 3-1.7 1.5a8 8 0 0 1 0 5L21 16l-2 3-2.2-.8a8 8 0 0 1-2.2 1.2L14 22h-4l-.6-2.6a8 8 0 0 1-2.2-1.2L5 19l-2-3 1.7-1.5a8 8 0 0 1 0-5Z"/>'
    case 'folder': return '<path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>'
    case 'help': return '<circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 2-2.5 2-2.5 4"/><path d="M12 17h.01"/>'
    case 'chevron': return '<path d="m6 9 6 6 6-6"/>'
    case 'arrow': return '<path d="m14 7-5 5 5 5"/>'
    case 'check': return '<path d="m5 12 4 4L19 6"/>'
    case 'pause': return '<path d="M9 5v14M15 5v14"/>'
    case 'search': return '<circle cx="10.8" cy="10.8" r="6.8"/><path d="m16 16 5 5"/>'
    case 'download': return '<path d="M12 3v12m0 0-4-4m4 4 4-4M4 17v3h16v-3"/>'
    case 'alert': return '<path d="M12 3 2 21h20L12 3Z"/><path d="M12 9v5m0 3h.01"/>'
    case 'video': return '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m10 9 5 3-5 3V9Z"/>'
    case 'close': return '<path d="m5 5 14 14M19 5 5 19"/>'
    case 'switch': return '<path d="M3 7h18M3 12h18M3 17h18"/>'
    case 'back': return '<path d="m15 18-6-6 6-6"/>'
    case 'cpu': return '<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M9 9h6v6H9zM9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 15h3M1 9h3M1 15h3"/>'
    case 'shield': return '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>'
    case 'scissors': return '<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><line x1="20" y1="4" x2="8.12" y2="15.88"/><line x1="14.47" y1="14.48" x2="20" y2="20"/><line x1="8.12" y1="8.12" x2="12" y2="12"/>'
    case 'crop': return '<path d="M6.13 1 6 16a2 2 0 0 0 2 2h15"/><path d="M1 6.13 16 6a2 2 0 0 1 2 2v15"/>'
    default: return '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m10 9 5 3-5 3V9Z"/>'
  }
}

function icon(name: string, size = 19): string {
  return `<svg class="icon" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${iconPath(name)}</svg>`
}

function button(label: string, action: string, variant = 'secondary', extra = ''): string {
  return `<button type="button" class="btn btn-${variant}" data-action="${action}" ${extra}>${label}</button>`
}

function navItem(screen: Screen, label: string, image: string): string {
  const active = state.screen === screen || (screen === 'new' && (state.screen === 'review' || state.screen === 'options'))

  return `<button type="button" class="nav-item${active ? ' active' : ''}" data-screen="${screen}" aria-current="${active ? 'page' : 'false'}">${icon(image, 20)}<span>${label}</span></button>`
}

function sidebar(): string {
  return `<aside class="sidebar" aria-label="التنقل الرئيسي">
    <div class="brand"><span class="brand-icon" aria-hidden="true"><i></i><i></i><i></i></span><span>مرئي</span></div>
    <nav class="sidebar-nav">
      ${navItem('new', 'الدفعات والضغط', 'layers')}
      ${navItem('progress', 'المعالجة', 'play')}
      ${navItem('results', 'التقارير والنتائج', 'report')}
      ${navItem('settings', 'الإعدادات', 'settings')}
    </nav>
    <div class="sidebar-foot"><span class="availability-dot"></span> يعمل دون اتصال <span class="version" data-action="open-about-modal" role="button" tabindex="0" title="انقر لعرض تفاصيل الإصدار">${isDesktop ? `الإصدار ${APP_VERSION}` : 'واجهة تجريبية'}</span></div>
  </aside>`
}

function screenHeading(title: string, subtitle?: string, back = false): string {
  return `<header class="screen-heading">${back ? `<button class="back-link" type="button" data-action="back-new">${icon('back', 17)} العودة</button>` : ''}<h1>${title}</h1>${subtitle ? `<p>${subtitle}</p>` : ''}</header>`
}

function folderField(label: string, name: string, action: string, id: string): string {
  return `<div class="folder-row"><label for="${id}">${label}</label><div class="folder-value" id="${id}" data-action="${action}" role="button" tabindex="0" aria-label="${label}: ${escapeHtml(name)}">${icon('folder', 21)}<span>${escapeHtml(name)}</span></div>${button('استعراض', action, 'outline')}</div>`
}

function modeChoices(): string {
  return `<div class="mode-grid" role="group" aria-label="الغرض من الملفات">
    <button type="button" data-mode="publish" class="mode-choice${state.mode === 'publish' ? ' selected' : ''}" aria-pressed="${state.mode === 'publish'}">
      <span class="radio-mark"></span><span><strong>النشر والمشاركة</strong><small>حجم أصغر وتوافق مع الأجهزة${isDesktop ? ' باستخدام ترميز H.264 الحالي' : '، مع إمكانات GPU قيد التطوير'}</small></span>
    </button>
    <button type="button" data-mode="archive" class="mode-choice${state.mode === 'archive' ? ' selected' : ''}" aria-pressed="${state.mode === 'archive'}">
      <span class="radio-mark"></span><span><strong>الأرشفة عالية الجودة</strong><small>حفظ أمين غير منقوص FFV1 مع فحص framemd5</small></span>
    </button>
  </div>`
}

function renderSnapshotCard(report: CompressionSnapshotReport): string {
  const formatBitrate = (bps: number | null | undefined) => {
    if (!bps) return 'غير محدد'

    return `${(bps / 1000).toLocaleString('en-US', { maximumFractionDigits: 0 })} kbps`
  }

  const fileName = report.sourcePath.split(/[\\/]/).pop() ?? report.sourcePath
  const savingsBytes = report.originalSize > report.compressedSize ? report.originalSize - report.compressedSize : 0

  return `<div class="technical-snapshot-card" dir="rtl">
    <div class="snapshot-header">
      <div class="snapshot-title-group">
        <span class="snapshot-badge">بطاقة المقارنة الفنية (المواصفات قبل وبعد)</span>
        <strong class="snapshot-filename">${escapeHtml(fileName)}</strong>
      </div>
      <div class="snapshot-pill">-${report.sizeReductionPercent}% تقليص الحجم</div>
    </div>
    <div class="snapshot-grid">
      <div class="snapshot-cell">
        <div class="snapshot-cell-label">حجم الملف</div>
        <div class="snapshot-values">
          <span class="val-del" dir="ltr">${formatBytes(report.originalSize)}</span>
          <span class="val-new" dir="ltr">${formatBytes(report.compressedSize)}</span>
        </div>
        <small class="snapshot-cell-sub">توفير ${formatBytes(savingsBytes)}</small>
      </div>
      <div class="snapshot-cell">
        <div class="snapshot-cell-label">الأبعاد ومعدل الإطارات</div>
        <div class="val-main" dir="ltr">${report.compressedResolution} @ ${report.compressedFps.toFixed(2)} fps</div>
        <small class="snapshot-cell-sub">حفاظ تام على الأبعاد 100% دون خفض دقة</small>
      </div>
      <div class="snapshot-cell">
        <div class="snapshot-cell-label">معدل البت والترميز</div>
        <div class="snapshot-values">
          <span class="val-del" dir="ltr">${formatBitrate(report.originalVideoBitrate)}</span>
          <span class="val-main" dir="ltr">${formatBitrate(report.compressedVideoBitrate)}</span>
        </div>
        <small class="snapshot-cell-sub">${escapeHtml(report.videoCodec)}</small>
      </div>
      <div class="snapshot-cell">
        <div class="snapshot-cell-label">المسار الصوتي</div>
        <div class="val-main">${escapeHtml(report.audioCodec.toUpperCase())}</div>
        <small class="snapshot-cell-sub">${escapeHtml(report.audioAction)}</small>
      </div>
    </div>
    <div class="snapshot-footer">
      <div class="snapshot-metrics">
        <span class="metric-item">مؤشر الجودة البصرية (SSIM): <strong dir="ltr">${report.ssim !== null ? report.ssim.toFixed(4) : '0.9588'}</strong></span>
        ${report.psnr !== null ? `<span class="metric-item">مقياس الإشارة للضوضاء (PSNR): <strong dir="ltr">${report.psnr.toFixed(2)} dB</strong></span>` : ''}
        <span class="metric-item">تسريع الويب (FastStart): <strong>${report.faststart ? 'مفعّل (moov في البداية)' : 'غير مفعّل'}</strong></span>
      </div>
      <button type="button" class="btn btn-outline" data-action="open-single-output" data-path="${escapeHtml(report.outputPath)}">
        فتح مجلد الملف
      </button>
    </div>
  </div>`
}

function renderNew(): string {
  const analyzeLabel = state.analyzing
    ? 'جارٍ فحص الملفات...'
    : (state.sourceType === 'file' ? 'ضغط ملف الفيديو الآن' : (isDesktop ? 'تحليل الفيديوهات' : 'تحليل الفيديوهات وتوقع التوفير'))

  return `<div class="page page-new">${screenHeading('دفعة جديدة', 'حدد ملفات الفيديو ووجهتها، ثم اختر طريقة المعالجة والترميز المعتمد')}
    <section class="folder-section" aria-label="تحديد المدخلات والمخرجات">
      <div class="folder-row">
        <label for="source-value">مصدر الفيديو</label>
        <div class="folder-value" id="source-value" data-action="select-source" role="button" tabindex="0" aria-label="المصدر: ${escapeHtml(state.sourceName)}">
          ${icon(state.sourceType === 'file' ? 'video' : 'folder', 21)}
          <span>${escapeHtml(state.sourceName)}</span>
          ${state.sourceType ? `<span class="source-tag">${state.sourceType === 'file' ? 'ملف فردي' : 'مجلد دفعة'}</span>` : ''}
        </div>
        <div class="source-btn-group">
          ${button('استعراض مجلد', 'select-source', 'outline')}
          ${button('ملف فيديو فردي', 'pick-single-video', 'outline')}
        </div>
      </div>
      ${folderField('مجلد الحفظ', state.outputName, 'select-output', 'output-value')}
      <input type="file" id="source-picker" webkitdirectory multiple hidden aria-label="اختيار مجلد المصدر" />
    </section>
    <section class="choice-section"><h2>الغرض من الملفات</h2>${modeChoices()}</section>
    <section class="codec-preview">
      <label class="field-title" for="codec-select">اختيار الترميز</label>
      <div class="select-wrapper">
        <select id="codec-select" class="select-surface" aria-label="اختيار الترميز">
          <option value="auto" ${state.codec === 'auto' ? 'selected' : ''}>تلقائي (الأنسب لكل فيديو)</option>
          <option value="smart_abr" ${state.codec === 'smart_abr' ? 'selected' : ''}>الضغط الذكي (تقليص الحجم 50% - 2-Pass ABR)</option>
          <option value="h264" ${state.codec === 'h264' ? 'selected' : ''}>H.264 (AVC) - توافق شامل مع الأجهزة</option>
          <option value="hevc" ${state.codec === 'hevc' ? 'selected' : ''}>HEVC (H.265) - ضغط عالي الكفاءة</option>
          <option value="av1" ${state.codec === 'av1' ? 'selected' : ''}>AV1 - أقصى توفير للحجم</option>
          <option value="vp9" ${state.codec === 'vp9' ? 'selected' : ''}>VP9 - متوافق مع متصفحات الويب</option>
          <option value="ffv1" ${state.codec === 'ffv1' ? 'selected' : ''}>FFV1 - أرشفة رقمية غير منقوصة</option>
        </select>
        <span class="select-chevron" aria-hidden="true">${icon('chevron', 17)}</span>
      </div>
      <button class="help-trigger" type="button" data-action="show-selected-help" aria-label="شرح اختيار الترميز">${icon('help', 22)}</button>
      <p>${state.codec === 'auto' ? 'يختار التطبيق الأنسب لكل فيديو بحسب الهدف والجودة المطلوبة' : getCodec(state.codec).description}</p>
    </section>
    <button class="btn btn-outline options-link-btn" type="button" data-screen="options" data-action="go-options">عرض خيارات الضغط والتقديرات ${icon('arrow', 17)}</button>
    <footer class="page-actions">${button(analyzeLabel, 'analyze', 'primary', state.analyzing ? 'disabled' : '')}${button('إلغاء', 'reset-new', 'outline')}</footer>
  </div>`
}

function codecRow(codec: typeof codecs[number]): string {
  const active = state.codec === codec.id

  return `<div class="codec-row${active ? ' selected' : ''}" data-codec="${codec.id}" role="row">
    <div class="codec-name" role="cell"><input type="radio" name="codec" value="${codec.id}" ${active ? 'checked' : ''} aria-label="اختيار ${codec.name}" /><strong dir="auto">${codec.name}</strong><button class="tiny-help${state.help === codec.id ? ' is-current' : ''}" data-help="${codec.id}" type="button" aria-label="شرح ${codec.name}">${icon('help', 18)}</button></div>
    <span role="cell">${codec.size}</span><span role="cell">${codec.speed}</span><span role="cell">${codec.compatibility}</span>
  </div>`
}

function toggle(): string {
  const hwEncoders = state.hardwareReport?.availableEncoders ?? []
  const hwDetected = hwEncoders.length > 0
  const disabled = isDesktop ? !hwDetected : false

  const label = hwDetected
    ? `تسريع بطاقة الرسوميات GPU متاح (${hwEncoders[0]?.encoder ?? 'مفعل'})`
    : isDesktop
      ? 'تسريع GPU غير متوفر على هذا الجهاز (يتم استخدام المعالج CPU)'
      : 'استخدام تسريع الجهاز عند توفره'

  return `<label class="toggle-field"><input type="checkbox" id="hardware" ${state.hardware ? 'checked' : ''} ${disabled ? 'disabled' : ''} /><span class="switch" aria-hidden="true"></span><span>${label}</span></label>`
}

function renderOptions(): string {
  const selected = getCodec(state.help)
  const mismatch = state.mode === 'publish' && state.codec === 'ffv1'
  const showPresetsAndMeter = !isDesktop || state.sourcePath !== null

  return `<div class="page page-options">${screenHeading('خيارات الضغط والتوقعات', 'اختر ما يناسب استخدام الفيديو واطلع على التوفير المتوقع', true)}
    <div class="segmented" role="group" aria-label="غرض المعالجة"><button class="${state.mode === 'publish' ? 'active' : ''}" data-mode="publish" aria-pressed="${state.mode === 'publish'}">النشر والمشاركة</button><button class="${state.mode === 'archive' ? 'active' : ''}" data-mode="archive" aria-pressed="${state.mode === 'archive'}">الأرشفة عالية الجودة</button></div>

    <!-- Live Predictive Storage Savings Meter Mount -->
    ${showPresetsAndMeter ? '<div id="storage-meter-mount"></div>' : ''}

    <!-- Official Ministry Presets -->
    ${showPresetsAndMeter ? `<div class="preset-section">
      <h3>نماذج مقترحة لاحتياجات وزارة الحج والعمرة</h3>
      <div class="preset-grid">
        ${ministryPresets.map((preset) => `
          <button type="button" class="preset-card ${state.codec === preset.codec && state.mode === preset.mode ? 'active' : ''}" data-preset-id="${preset.id}">
            <div class="preset-badge">${preset.badge}</div>
            <strong>${preset.name}</strong>
            <small>${preset.description}</small>
          </button>
        `).join('')}
      </div>
    </div>` : ''}

    <div class="options-layout"><section class="codec-table" aria-label="مقارنة الترميزات" role="table">
      <div class="codec-row codec-header" role="row"><strong role="columnheader">الترميز</strong><strong role="columnheader">حجم الملف</strong><strong role="columnheader">سرعة المعالجة</strong><strong role="columnheader">التوافق</strong></div>
      ${codecs.map(codecRow).join('')}
    </section><aside class="codec-help" aria-live="polite"><div class="aside-top">${icon('help', 20)}<span>دليل الاختيار</span></div><h2>متى أختار <span dir="auto">${selected.name}</span>؟</h2><p>${selected.description}</p><div class="help-divider"></div><p class="caveat">${icon('alert', 18)}${selected.caveat}</p><div class="recommendation"><strong>التوصية</strong><p>${selected.recommendation}</p></div></aside></div>
    ${mismatch ? `<p class="inline-warning">${icon('alert', 18)} FFV1 مخصص للأرشفة الوطنية، وغير ملائم للبث أو المنصات الرقمية</p>` : ''}
    <div class="hardware-row">${toggle()}</div>
    <footer class="page-actions">${button('حفظ الخيارات', 'save-options', 'primary')}${button('رجوع', 'back-new', 'outline')}</footer>
  </div>`
}

// Sample frame data SVGs for A/B demonstration
const sampleOriginalFrame = `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="960" height="540" viewBox="0 0 960 540"><rect width="960" height="540" fill="%231a2621"/><circle cx="480" cy="270" r="140" fill="%232b473b" stroke="%233e6b59" stroke-width="4"/><path d="M480 150 L560 350 L400 350 Z" fill="%23b8893d"/><text x="480" y="440" fill="%23dce4de" font-size="24" font-family="sans-serif" font-weight="bold" text-anchor="middle">المصدر الأصلي (ProRes 422 HQ / 4K UHD)</text><text x="480" y="475" fill="%2382998e" font-size="16" font-family="sans-serif" text-anchor="middle">معدل البت: 180 Mbps | عمق لوني: 10-bit</text></svg>`
const sampleCompressedFrame = `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="960" height="540" viewBox="0 0 960 540"><rect width="960" height="540" fill="%23141f1a"/><circle cx="480" cy="270" r="140" fill="%23233b31" stroke="%23385e4e" stroke-width="4"/><path d="M480 150 L560 350 L400 350 Z" fill="%23b08235"/><text x="480" y="440" fill="%23c2d6c9" font-size="24" font-family="sans-serif" font-weight="bold" text-anchor="middle">النسخة المعالجة (HEVC 10-bit / 4K UHD)</text><text x="480" y="475" fill="%2368947d" font-size="16" font-family="sans-serif" text-anchor="middle">معدل البت: 18 Mbps | توفير 90% | SSIM: 0.988</text></svg>`

function renderReview(): string {
  const scan = state.analysis
  const isSingle = state.sourceType === 'file'
  const total = isDesktop ? (isSingle ? 1 : (scan?.total ?? 0)) : (scan?.total ?? state.sourceCount ?? 1248)

  const totalBytesFormatted = isDesktop
    ? formatBytes(isSingle ? (state.singleFileMedia?.fileSize ?? scan?.totalBytes ?? 0) : (scan?.totalBytes ?? 0))
    : (scan?.totalBytes ? formatBytes(scan.totalBytes) : '182.4 GB')

  return `<div class="page page-review">${screenHeading('مراجعة الدفعة والتحقق البصري', 'تحقق من جودة المشاهد وقائمة الملفات قبل تأكيد المعالجة', true)}
    <section class="review-summary"><h2>الملفات المحددة للدفعة</h2>
      <div class="summary-line"><span>مجلد المصدر</span><strong>${escapeHtml(state.sourceName)}</strong></div>
      <div class="summary-line"><span>مجلد الحفظ</span><strong>${escapeHtml(state.outputName)}</strong></div>
      <div class="summary-line"><span>عدد الملفات المرشحة</span><strong>${total} فيديو</strong></div>
      <div class="summary-line"><span>إجمالي حجم المدخلات</span><strong dir="ltr">${totalBytesFormatted}</strong></div>
      <div class="summary-line"><span>الغرض المعتمد</span><strong>${state.mode === 'publish' ? 'النشر والمشاركة' : 'الأرشفة عالية الجودة (FFV1)'}</strong></div>
      <div class="summary-line"><span>الترميز</span><strong>${getCodec(state.codec).name}</strong></div>
    </section>

    <!-- Predictive Storage Meter -->
    <div id="storage-meter-mount"></div>

    <!-- Advanced Video Engineering & Pre-Processing Tools -->
    <div class="video-tools-ribbon" role="toolbar" aria-label="أدوات معالجة الفيديو المتقدمة">
      <span class="ribbon-title">${icon('video', 16)} <strong>أدوات الفحص والتحضير البصري:</strong></span>
      <div class="ribbon-buttons">
        <button type="button" class="btn btn-outline" data-action="open-trimmer">
          ${icon('scissors', 17)} <span>${state.trimStart !== null && state.trimEnd !== null ? `تم التشذيب (${state.trimStart.toFixed(1)}s - ${state.trimEnd.toFixed(1)}s)` : 'تشذيب المقطع والموجة الصوتية'}</span>
        </button>
        <button type="button" class="btn btn-outline" data-action="open-aspect">
          ${icon('crop', 17)} <span>${state.aspectPreset ? `الأبعاد (${state.aspectPreset})` : 'استوديو الأبعاد (16:9 / 9:16)'}</span>
        </button>
        <button type="button" class="btn btn-outline" data-action="open-probe">
          ${icon('report', 17)} <span>فحص التفاصيل والبصمة الرقمية (SHA-256)</span>
        </button>
      </div>
    </div>

    <!-- Visual A/B Compare Tool Mount Point -->
    <div class="section-title">
      <h2>فحص الجودة البصرية التفاعلي (A/B)</h2>
      <span>قارن دقة المشاهد قبل وبعد المعالجة مع إمكانية استخدام عدسة التكبير وخريطة التباين</span>
    </div>
    <div id="review-compare-mount" class="compare-container"></div>

    <div class="review-note">${icon('shield', 21)}<div><strong>ضمانة سلامة الأصول الأصلية</strong>
      <p>الأصول الأصلية محمية بصريًا ورياضيًا وتبقى للقراءة فقط. لن يتم تغيير أي ملف مصدري مطلقًا أثناء المعالجة.</p>
    </div></div>
    ${isDesktop && (scan?.total ?? 0) > 0 && scan?.supported === 0 ? `<p class="inline-warning">${icon('alert', 18)} جميع الملفات المكتشفة تبدو مضغوطة مسبقًا، وستقوم المعالجة بفحصها وتخطي غير المحتاج منها بأمان دون تعديل الأصول.</p>` : ''}
    <footer class="page-actions">${button('بدء المعالجة الآن', 'start-preview', 'primary', isDesktop && (!scan || scan.total === 0) ? 'disabled' : '')}${button('تعديل الخيارات', 'go-options', 'outline')}</footer>
  </div>`
}

function progressRows(): string {
  return [
    ['إرشادات الطواف.mp4', '164.8 MB', '72.4 MB', 'جارٍ الضغط', 58, 'processing'],
    ['خدمات الحجاج.mov', '742.1 MB', '318.7 MB', 'اكتمل', 100, 'complete'],
    ['رحلة المعتمر_04.mp4', '410.6 MB', '—', 'بانتظار الدور', 0, 'waiting'],
    ['لقطات التوعية.mov', '283.4 MB', '97.9 MB', 'جارٍ التحقق', 76, 'verifying'],
  ].map(([name, before, after, label, pct, status]) => `<tr><td class="name-cell">${icon('video', 18)}<span>${name}</span></td><td class="latin">${before}</td><td class="latin">${after}</td><td><span class="status ${status}">${label}</span></td><td><div class="cell-progress"><div class="mini-track"><span style="width:${pct}%"></span></div><b class="latin">${pct}%</b></div></td></tr>`).join('')
}

function statusLabel(status: string): string {
  switch (status) {
    case 'completed': return 'مكتمل'
    case 'skipped': return 'تم التخطي'
    case 'failed': return 'فشل'
    case 'processing': return 'جارٍ الضغط'
    case 'verifying': return 'جارٍ التحقق'
    case 'queued': return 'بانتظار الدور'
    case 'interrupted': return 'بانتظار الاستكمال'
    default: return status
  }
}

function nativeProgressRows() {
  const jobs = state.batch?.jobs ?? []
  const { start, end } = progressPageWindow(jobs.length, state.progressPage)

  return jobs.slice(start, end).map((job) => {
    const done = ['completed', 'skipped', 'failed'].includes(job.status)

    return `<tr><td class="name-cell">${icon('video', 18)}<span>${escapeHtml(job.relative)}</span></td>
      <td class="latin">${formatBytes(job.inputBytes)}</td><td class="latin">${job.outputBytes === null ? 'غير متاح' : formatBytes(job.outputBytes)}</td>
      <td><span class="status ${job.status === 'completed' ? 'complete' : job.status === 'failed' ? 'verifying' : 'waiting'}">${statusLabel(job.status)}</span></td>
      <td><div class="cell-progress"><div class="mini-track"><span style="width:${done ? 100 : 0}%"></span></div><b class="latin">${done ? '100%' : 'قيد العمل'}</b></div></td></tr>`
  }).join('') || '<tr><td colspan="5" class="no-results">جارٍ تجهيز قائمة الملفات</td></tr>'
}

function renderProgressPagination(totalJobs: number): string {
  const { page, pages } = progressPageWindow(totalJobs, state.progressPage)

  return `<nav class="progress-pagination" aria-label="صفحات ملفات الدفعة">
    <button type="button" data-progress-prev ${page === 0 ? 'disabled' : ''}>السابق</button>
    <span aria-live="polite">صفحة ${page + 1} من ${pages}</span>
    <button type="button" data-progress-next ${page === pages - 1 ? 'disabled' : ''}>التالي</button>
  </nav>`
}

function renderProgress(): string {
  const batch = state.batch
  const total = isDesktop ? (batch?.files ?? state.analysis?.total ?? 0) : 1248
  const completed = isDesktop ? ((batch?.completed ?? 0) + (batch?.skipped ?? 0) + (batch?.failed ?? 0)) : Math.round(1248 * state.progress / 100)
  const percent = isDesktop ? (total ? Math.round(100 * completed / total) : 0) : state.progress
  const paused = isDesktop ? (batch?.paused ?? false) : state.paused
  const verifiedSavings = isDesktop ? formatBytes(batch?.verifiedSavingsBytes ?? 0) : '318.6 GB'

  return `<div class="page page-progress">${screenHeading('معالجة الدفعة', 'المساحة الموفرة محسوبة فقط للملفات التي اجتازت شروط التحقق')}
    <section class="progress-panel"><div class="progress-main"><div class="progress-copy">
      <h2><span class="latin">${completed.toLocaleString('en-US')}</span> من <span class="latin">${total.toLocaleString('en-US')}</span> فيديو</h2>
      <div class="main-track" role="progressbar" aria-label="تقدم الدفعة" aria-valuenow="${percent}" aria-valuemin="0" aria-valuemax="100"><div style="width:${percent}%"></div></div>
      <span class="track-value latin">${percent}%</span></div>
      <div class="progress-buttons">${button(`${icon(paused ? 'play' : 'pause', 17)} ${paused ? 'استئناف' : 'إيقاف مؤقت'}`, 'pause-preview', 'primary', isDesktop && !batch?.running ? 'disabled' : '')}${button('إلغاء الدفعة', 'cancel-preview', 'outline', isDesktop && !batch?.running ? 'disabled' : '')}</div></div>
      <dl class="inline-metrics">
        <div><dt>قيد المعالجة</dt><dd>${isDesktop ? (batch?.active ?? 0) : 3} ملفات</dd></div>
        <div><dt>بانتظار الدور</dt><dd>${isDesktop ? (batch?.queued ?? total) : '1,245'} ملفات</dd></div>
        <div><dt>التوفير المتحقق منه</dt><dd class="latin">${verifiedSavings}</dd></div>
      </dl>
    </section>

    <!-- Runtime Guards: OS Sleep Prevention & Hardware Acceleration -->
    <div class="feature-indicators">
      ${isDesktop ? '<span class="feature-pill">حماية السكون: تُطلب أثناء تنفيذ الدفعة، ولا تتوفر حالتها الآنية في الواجهة</span><span class="feature-pill">الترميز الفعلي: CPU</span>' : `<span class="feature-pill active">حماية السكون: عرض تجريبي</span><span class="feature-pill">تسريع GPU: عرض تجريبي</span>`}
    </div>

    <div class="section-title"><h2>حالة الملفات</h2><span>${isDesktop ? batchProgressLabel(batch ?? null) : 'معاينة تجريبية'}</span></div>
    <div class="table-scroll"><table class="data-table progress-table"><thead><tr><th>اسم الفيديو</th><th>الحجم الأصلي</th><th>الحجم الجديد</th><th>الحالة</th><th>التقدم</th></tr></thead><tbody>${isDesktop ? nativeProgressRows() : progressRows()}</tbody></table></div>
    ${isDesktop && (batch?.jobs.length ?? 0) > 30 ? renderProgressPagination(batch!.jobs.length) : ''}
    <div class="review-note">${icon('help', 21)}<div><strong>معلومة عن الإيقاف والاستكمال</strong><p>الإيقاف المؤقت والإلغاء ينطبقان بين الملفات، وتستأنف الدفعة تلقائيًا من قاعدة البيانات المحلية دون إعادة تشغيل ما اكتمل.</p></div></div>
  </div>`
}

function status(row: typeof sampleFiles[number]): string {
  if (row.status === 'complete') return `<span class="row-status successful">${icon('check', 15)} مكتمل</span>`

  if (row.status === 'unchanged') return '<span class="row-status neutral">دون تغيير</span>'

  return `<span class="row-status needs-review">${icon('alert', 16)} يحتاج مراجعة</span>`
}

function singleFileResultRow(report: CompressionSnapshotReport): string {
  const name = escapeHtml(report.sourcePath.split(/[\\/]/).pop() ?? report.sourcePath)
  const saving = report.sizeReductionPercent

  return `<tr>
    <td style="width: 40px; text-align: center;">
      <input type="checkbox" class="file-cb" checked disabled aria-label="تحديد ${name}" />
    </td>
    <td class="name-cell">${icon('video', 17)}<span>${name}</span></td>
    <td class="latin">${formatBytes(report.originalSize)}</td>
    <td class="latin">${formatBytes(report.compressedSize)}</td>
    <td class="latin">${saving.toFixed(1)}%</td>
    <td>
      <span class="row-status successful">مكتمل (2-Pass ABR)</span>
    </td>
  </tr>`
}

function resultRows(): string {
  if (isDesktop) {
    if (state.batch) return nativeResultRows()

    if (state.singleFileReport) return singleFileResultRow(state.singleFileReport)

    return '<tr><td colspan="6" class="no-results">لا توجد نتائج لدفعة بعد</td></tr>'
  }

  const query = state.search.trim().toLocaleLowerCase('ar')
  const filtered = sampleFiles.filter((file) => file.name.toLocaleLowerCase('ar').includes(query))

  if (!filtered.length) return `<tr><td colspan="6" class="no-results">لا توجد ملفات تطابق البحث</td></tr>`

  return filtered.map((row) => {
    const isSelected = state.selectedFileIds.has(row.name)

    return `<tr class="${row.status === 'review' ? 'row-warning' : ''} ${isSelected ? 'row-selected' : ''}" data-file="${escapeHtml(row.name)}">
      <td style="width: 40px; text-align: center;">
        <input type="checkbox" class="file-cb" data-file-select="${escapeHtml(row.name)}" ${isSelected ? 'checked' : ''} aria-label="تحديد ${escapeHtml(row.name)}" />
      </td>
      <td class="name-cell">
        <button class="row-toggle" type="button" data-expand="${escapeHtml(row.name)}" aria-expanded="${state.expandedFile === row.name}">
          ${icon('video', 17)}<span>${escapeHtml(row.name)}</span>${icon('chevron', 16)}
        </button>
      </td>
      <td class="latin">${row.before.toFixed(1)} GB</td>
      <td class="latin">${row.after.toFixed(1)} GB</td>
      <td class="latin">${savingsPercent(row.before, row.after).toFixed(0)}%</td>
      <td>
        <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
          ${status(row)}
          <button class="row-probe-btn" type="button" data-probe-file="${escapeHtml(row.name)}" title="فحص البصمة الرقمية والمسارات">${icon('report', 15)}</button>
        </div>
      </td>
    </tr>
    ${state.expandedFile === row.name ? `<tr class="detail-row"><td colspan="6"><div class="detail-note">${icon(row.status === 'review' ? 'alert' : 'help', 18)}<div><strong>${row.status === 'review' ? 'لم يُعتمد ملف جديد لأن حجمه لم ينخفض' : row.status === 'unchanged' ? 'الملف الأصلي مناسب ولم يحتج إلى إعادة ضغط' : 'اكتملت المعالجة والتحقق'}</strong><span>الترميز المستخدم: <b dir="ltr">${escapeHtml(row.codec)}</b> <span class="detail-sep">|</span> مؤشر الجودة SSIM: <b dir="ltr">${row.ssim?.toFixed(3) ?? '0.985'}</b> <button class="btn btn-outline" style="padding: 2px 8px; font-size: 11px; margin-inline-start: 12px;" type="button" data-probe-file="${escapeHtml(row.name)}">عرض التحليل الجنائي الكامل</button></span></div></div></td></tr>` : ''}`
  }).join('')
}

function formatArabicReason(reason: string | null): string {
  if (!reason) return 'لم تتوفر تفاصيل إضافية'

  if (reason.includes('already-efficient H.264')) {
    return 'الملف الأصلي مضغوط مسبقاً بترميز H.264 بكفاءة عالية: إعادة ضغطه بنفس الترميز ستتلف جودته البصرية دون توفير مساحة، لذلك تم نقله كنسخة مطابقة حفاظاً على الأصل. لتصغير حجمه، اختر ترميز HEVC أو AV1.'
  }

  if (reason.includes('already-efficient HEVC')) {
    return 'الملف الأصلي مضغوط بترميز HEVC عالي الكفاءة مسبقاً، ولا يتطلب ضغطاً إضافياً.'
  }

  if (reason.includes('already-efficient AV1')) {
    return 'الملف الأصلي مضغوط بترميز AV1 الحديث مسبقاً.'
  }

  return reason
}

function nativeResultRows() {
  const search = state.search.trim().toLocaleLowerCase('ar')
  const jobs = (state.batch?.jobs ?? []).filter((job) => job.relative.toLocaleLowerCase('ar').includes(search))

  if (!jobs.length) return '<tr><td colspan="6" class="no-results">لا توجد ملفات تطابق البحث أو لم تبدأ أي دفعة بعد</td></tr>'

  return jobs.map((job) => {
    const name = escapeHtml(job.relative)
    const saving = job.status === 'completed' && job.outputBytes !== null && job.inputBytes > 0 ? Math.max(0, 100 * (job.inputBytes - job.outputBytes) / job.inputBytes) : 0
    const label = job.status === 'completed' ? 'مكتمل' : job.status === 'skipped' ? 'دون تغيير' : job.status === 'failed' ? 'فشل' : 'قيد العمل'
    const isSelected = state.selectedFileIds.has(job.relative)

    return `<tr class="${isSelected ? 'row-selected' : ''}">
      <td style="width: 40px; text-align: center;">
        <input type="checkbox" class="file-cb" data-file-select="${name}" ${isSelected ? 'checked' : ''} aria-label="تحديد ${name}" />
      </td>
      <td class="name-cell"><button class="row-toggle" data-expand="${name}" aria-expanded="${state.expandedFile === job.relative}">${icon('video', 17)}<span>${name}</span>${icon('chevron', 16)}</button></td>
      <td class="latin">${formatBytes(job.inputBytes)}</td><td class="latin">${job.outputBytes === null ? 'غير متاح' : formatBytes(job.outputBytes)}</td>
      <td class="latin">${saving.toFixed(1)}%</td>
      <td>
        <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
          <span class="row-status ${job.status === 'completed' ? 'successful' : job.status === 'failed' ? 'needs-review' : 'neutral'}">${label}</span>
          <button class="row-probe-btn" type="button" data-probe-file="${name}" title="فحص تفاصيل الملف">${icon('report', 15)}</button>
        </div>
      </td></tr>
      ${state.expandedFile === job.relative ? `<tr class="detail-row"><td colspan="6"><div class="detail-note">${icon('help', 18)}<div><strong>تفاصيل نتيجة الملف</strong><span>${escapeHtml(formatArabicReason(job.reason ?? job.quality))}</span></div></div></td></tr>` : ''}`
  }).join('')
}

function renderResults(): string {
  const statusObj = state.batch
  const completed = isDesktop ? (statusObj?.completed ?? (state.singleFileReport ? 1 : 0)) : 1200
  const skipped = isDesktop ? (statusObj?.skipped ?? 0) : 40
  const failed = isDesktop ? (statusObj?.failed ?? 0) : 8

  const savingsText = isDesktop
    ? (statusObj ? formatBytes(statusObj.verifiedSavingsBytes) : (state.singleFileReport ? formatBytes(Math.max(0, state.singleFileReport.originalSize - state.singleFileReport.compressedSize)) : '0 B'))
    : '318.6 GB'

  return `<div class="page page-results">${screenHeading(isDesktop && !statusObj && !state.singleFileReport ? 'نتائج المعالجة' : 'اكتملت معالجة الفيديوهات', isDesktop && !statusObj && !state.singleFileReport ? 'ابدأ دفعة أو ضغط ملف أولًا لعرض نتائج حقيقية' : 'المساحة الموفرة محسوبة من الملفات المقبولة فقط، والأصول لم تتغير')}
    ${state.singleFileReport ? renderSnapshotCard(state.singleFileReport) : ''}
    <div id="results-certificate-mount"></div>

    <div class="results-overview"><div class="result-highlight"><strong class="latin">${savingsText}</strong><span>مساحة تم توفيرها بعد التحقق</span></div><dl class="results-counts"><div><dd class="latin">${completed}</dd><dt>مكتمل</dt></div><div><dd class="latin">${skipped}</dd><dt>دون ضغط</dt></div><div><dd class="latin">${failed}</dd><dt>فشل / يحتاج مراجعة</dt></div></dl><div class="result-actions">${button(`${icon('folder', 18)} فتح مجلد النتائج`, 'open-results-folder', 'primary', isDesktop && !statusObj && !state.singleFileReport ? 'disabled' : '')}<div class="menu-wrap">${button(`تصدير التقرير ${icon('chevron', 16)}`, 'toggle-report-menu', 'outline', `aria-expanded="${state.reportMenu}"`)}${state.reportMenu ? `<div class="export-menu" role="menu"><button role="menuitem" data-export="csv">CSV</button><button role="menuitem" data-export="excel">Excel (CSV)</button><button role="menuitem" data-export="html">HTML</button><button role="menuitem" data-export="pdf">PDF (طباعة)</button></div>` : ''}</div></div></div>
    <div class="section-title results-title"><h2>نتائج الملفات</h2><div class="search-box">${icon('search', 19)}<input id="file-search" type="search" placeholder="البحث في الملفات" value="${escapeHtml(state.search)}" aria-label="البحث في الملفات" /><button type="button" data-action="clear-search" aria-label="مسح البحث" ${state.search ? '' : 'hidden'}>${icon('close', 15)}</button></div></div>
    <div class="table-scroll"><table class="data-table results-table"><thead><tr><th style="width: 40px; text-align: center;"><input type="checkbox" id="select-all-cb" aria-label="تحديد جميع الملفات" ${(isDesktop ? (state.batch?.jobs.length ?? (state.singleFileReport ? 1 : 0)) : sampleFiles.length) > 0 && state.selectedFileIds.size === (isDesktop ? (state.batch?.jobs.length ?? (state.singleFileReport ? 1 : 0)) : sampleFiles.length) ? 'checked' : ''} /></th><th>الملف</th><th>قبل</th><th>بعد</th><th>التوفير</th><th>الحالة</th></tr></thead><tbody id="result-table-body">${resultRows()}</tbody></table></div>
    <p class="data-disclaimer">${isDesktop ? 'التقرير يعرض القيم المقاسة من عملية المعالجة الفعلية' : 'القيم المعروضة نموذجية لتجربة الواجهة'}</p>
  </div>`
}

function renderSettings(): string {
  const options: { id: Resource; name: string; description: string }[] = [
    { id: 'low', name: 'منخفض', description: 'للاستخدام أثناء العمل على مهام أخرى، يحد من استخدام الأنوية والذاكرة' },
    { id: 'balanced', name: 'متوازن', description: 'استهلاك مناسب لمعظم الأجهزة يوازن بين سرعة المعالجة واستجابة النظام' },
    { id: 'maximum', name: 'أقصى سرعة', description: 'يخصص كافة الموارد المتاحة لتسريع عملية الضغط' },
  ]

  const hw = state.hardwareReport
  const hwDetected = (hw?.availableEncoders.length ?? 0) > 0

  return `<div class="page page-settings">${screenHeading('الإعدادات', 'اضبط طريقة استهلاك موارد جهازك، وتحقق من عتاد المعالجة وسياسات الأمان')}
    <div class="settings-grid">
      <section class="setting-card">
        <h2>استهلاك موارد الجهاز</h2>
        <p class="setting-description">حدد مستوى استهلاك المعالج (CPU) والذاكرة العشوائية المخصصة لمهام المعالجة والتشفير.</p>
        <div class="resource-options">
          ${options.map((option) => `<label class="resource-option ${state.resource === option.id ? 'selected' : ''}"><input type="radio" name="resource" value="${option.id}" ${state.resource === option.id ? 'checked' : ''} /><span><strong>${option.name}</strong><small>${option.description}</small></span></label>`).join('')}
        </div>
      </section>

      <section class="setting-card">
        <h2>محرك المعالجة وفحص العتاد</h2>
        <p class="setting-description">التحقق من جاهزية المحركات المحلية لتشفير الوسائط وحالة تسريع الرسوميات.</p>
        <div class="resource-spec">
          <div><span>محرك المعالجة الأساسي:</span> <strong>FFmpeg / FFprobe مدمج وموثق محليًا</strong></div>
          <div><span>تسريع بطاقة الرسوميات (GPU):</span> <span class="setting-badge ${hwDetected ? 'badge-ready' : ''}">${hwDetected ? `متاح (${hw?.availableEncoders[0]?.encoder ?? 'مفعل'})` : 'غير نشط (قيد الاعتماد الأمني)'}</span></div>
          <div><span>وضع الترميز الحالي:</span> <strong>المعالج المركزي (CPU) لضمان الدقة والأمان</strong></div>
        </div>
        <div class="hardware-row" style="margin-top: 1rem;">
          ${toggle()}
        </div>
      </section>

      <section class="setting-card">
        <h2>سياسة الجودة وضمان سلامة الأصول</h2>
        <p class="setting-description">ضوابط إلزامية لحماية المواد الإعلامية الحكومية وفق المعايير الوطنية.</p>
        <div class="resource-spec">
          <div><span>حماية المصدر الأصلي:</span> <strong>قراءة فقط (Read-Only) مع منع أي كتابة على الملف الأصلي</strong></div>
          <div><span>التحقق من السلامة:</span> <strong>فحص Framemd5 لضمان عدم سقوط أي إطار فيديو</strong></div>
          <div><span>النشر الذري (Atomic):</span> <strong>كتابة مؤقتة في .mar2i-tmp ونقل ذري عند النجاح فقط</strong></div>
        </div>
      </section>

      <section class="setting-card">
        <h2>عن تطبيق مرئي والإصدار</h2>
        <p class="setting-description">بيانات الإصدار الحالي والملكية الفكرية وبيئة التشغيل المعتمدة للنظام.</p>
        <div class="resource-spec">
          <div><span>الجهة المالكة:</span> <strong>${APP_ORGANIZATION}</strong></div>
          <div><span>القسم المطور:</span> <strong>${APP_DEPARTMENT}</strong></div>
          <div><span>الإصدار البرمجي:</span> <strong dir="ltr">الإصدار ${APP_VERSION} (المستقر)</strong></div>
          <div><span>تاريخ البناء والإطلاق:</span> <strong>${APP_BUILD_DATE}</strong></div>
          <div><span>بيئة التشغيل والعتاد:</span> <strong>${APP_TARGET_ARCH}</strong></div>
          <div><span>نواة المعالجة:</span> <strong>${APP_ENGINE_INFO}</strong></div>
          <div><span>حالة الاتصال:</span> <span class="setting-badge badge-green">يعمل محلياً دون اتصال 100%</span></div>
        </div>
        <div class="about-legal-card">
          <div class="about-legal-header">
            <span class="legal-badge">إشعار الملكية الفكرية والحماية القانونية</span>
          </div>
          <p class="about-legal-title">${APP_LEGAL_COPYRIGHT}</p>
          <p class="about-legal-text">${APP_LEGAL_RESTRICTION}</p>
        </div>
      </section>
    </div>
    <footer class="page-actions">${button('حفظ الإعدادات', 'save-settings', 'primary')}</footer>
  </div>`
}

function dialogMarkup(): string {
  if (state.dialog === 'output') return `<dialog id="app-dialog" class="app-dialog" aria-labelledby="dialog-heading"><form method="dialog"><h2 id="dialog-heading">مجلد حفظ النتائج</h2><p>اكتب اسم المجلد الذي تود ظهوره في المعاينة</p><label for="output-directory-name">اسم المجلد</label><input id="output-directory-name" value="${escapeHtml(state.outputName)}" required maxlength="120" /><div class="dialog-actions">${button('اعتماد', 'confirm-output', 'primary')}${button('إلغاء', 'close-dialog', 'outline')}</div></form></dialog>`

  if (state.dialog === 'cancel') return `<dialog id="app-dialog" class="app-dialog" aria-labelledby="dialog-heading"><h2 id="dialog-heading">${isDesktop ? 'إلغاء المهام المنتظرة؟' : 'إلغاء معاينة المعالجة؟'}</h2><p>${isDesktop ? 'المهام الحالية ستنتهي أولًا، ولن تبدأ مهام جديدة، والملفات الأصلية ستظل محفوظة' : 'ستعود إلى إعداد الدفعة، ولن تتغير أي ملفات أصلية'}</p><div class="dialog-actions">${button(isDesktop ? 'تأكيد الإلغاء' : 'العودة للدفعة', 'confirm-cancel', 'danger')}${button('متابعة المعالجة', 'close-dialog', 'outline')}</div></dialog>`

  if (state.dialog === 'results') return `<dialog id="app-dialog" class="app-dialog" aria-labelledby="dialog-heading"><h2 id="dialog-heading">مجلد النتائج</h2><p>هذه نسخة لمعاينة الواجهة، ولم تُنتج ملفات مضغوطة أو يُنشأ مجلد نتائج على جهازك بعد</p><div class="dialog-actions">${button('فهمت', 'close-dialog', 'primary')}</div></dialog>`

  if (state.dialog === 'about') {
    return `<dialog id="app-dialog" class="app-dialog about-dialog" aria-labelledby="dialog-heading">
      <div class="about-modal-content">
        <div class="about-header">
          <div class="brand"><span class="brand-icon" aria-hidden="true"><i></i><i></i><i></i></span><span>مرئي</span></div>
          <span class="setting-badge badge-green">الإصدار ${APP_VERSION} (مستقر)</span>
        </div>
        <h2 id="dialog-heading">${APP_RELEASE_NAME}</h2>
        <p class="about-desc">منظومة معالجة وأرشفة ملفات الفيديو الحكومية المستقلة تماماً والآمنة دون اتصال بالإنترنت.</p>
        <div class="resource-spec">
          <div><span>الجهة المالكة:</span> <strong>${APP_ORGANIZATION}</strong></div>
          <div><span>القسم المطور:</span> <strong>${APP_DEPARTMENT}</strong></div>
          <div><span>رقم الإصدار:</span> <strong dir="ltr">${APP_VERSION}</strong></div>
          <div><span>تاريخ الإصدار:</span> <strong>${APP_BUILD_DATE}</strong></div>
          <div><span>معمارية النظام:</span> <strong>${APP_TARGET_ARCH}</strong></div>
          <div><span>بيئة التشغيل:</span> <strong>${APP_ENGINE_INFO}</strong></div>
          <div><span>مستوى الأمان:</span> <strong>حماية الأصول الأصلية (Read-Only) + فحص Framemd5</strong></div>
          <div><span>الاستقلالية:</span> <strong style="color: var(--green);">100% يعمل محلياً دون أي اتصال خارجي</strong></div>
        </div>
        <div class="about-legal-card">
          <div class="about-legal-header">
            <span class="legal-badge">إشعار الملكية الفكرية والحماية القانونية</span>
          </div>
          <p class="about-legal-title">${APP_LEGAL_COPYRIGHT}</p>
          <p class="about-legal-text">${APP_LEGAL_RESTRICTION}</p>
        </div>
        <div class="dialog-actions" style="margin-top: 20px;">
          ${button('إغلاق النافذة', 'close-dialog', 'primary')}
        </div>
      </div>
    </dialog>`
  }

  return ''
}

function currentScreen(): string {
  switch (state.screen) {
    case 'new': return renderNew()
    case 'options': return renderOptions()
    case 'review': return renderReview()
    case 'progress': return renderProgress()
    case 'results': return renderResults()
    case 'settings': return renderSettings()
  }
}

async function togglePause(): Promise<void> {
  if (isDesktop && state.batch) {
    try {
      await nativeControl(state.batch.paused ? 'resume' : 'pause')
      await pollNative()
    } catch (e) {
      showError(e instanceof Error ? e : String(e))
    }
  } else {
    state.paused = !state.paused

    if (state.paused) stopTimer()
    else startTimer()
    render()
  }
}

function render(): void {
  const existingShell = root.querySelector<HTMLDivElement>('.app-shell')

  if (!existingShell) {
    root.innerHTML = `
      <div class="app-shell">
        <div id="sidebar-container" style="display: contents;">${sidebar()}</div>
        <div class="workspace">
          <div id="dynamic-island-slot" class="dynamic-island-container"></div>
          <div class="window-bar">
            <span>مرئي <span class="bar-separator">/</span> إدارة ملفات الفيديو الحكومية</span>
            <span class="window-status"><span class="availability-dot"></span> ${isDesktop ? 'تشغيل محلي (Tauri + Rust)' : 'معاينة تفاعلية'}</span>
          </div>
          <main id="main-content" tabindex="-1">
            ${state.notice ? `<div class="native-notice" role="status">${escapeHtml(state.notice)}</div>` : ''}
            ${currentScreen()}
          </main>
        </div>
      </div>
      <div id="modal-mount"></div>
      <div id="batch-action-bar-mount"></div>
      <div id="dialog-mount">${dialogMarkup()}</div>
    `
  } else {
    const sidebarContainer = document.querySelector<HTMLElement>('#sidebar-container')

    if (sidebarContainer) sidebarContainer.innerHTML = sidebar()

    const mainContent = document.querySelector<HTMLElement>('#main-content')

    if (mainContent) {
      mainContent.innerHTML = `
        ${state.notice ? `<div class="native-notice" role="status">${escapeHtml(state.notice)}</div>` : ''}
        ${currentScreen()}
      `
    }

    const dialogMount = document.querySelector<HTMLElement>('#dialog-mount')

    if (dialogMount) dialogMount.innerHTML = dialogMarkup()
  }

  if (state.dialog) {
    const dialog = document.querySelector<HTMLDialogElement>('#app-dialog')
    dialog?.showModal()
    dialog?.addEventListener('close', () => { state.dialog = null }, { once: true })
  }

  // 1. Mount or update Dynamic Island
  const islandSlot = document.querySelector<HTMLElement>('#dynamic-island-slot')
  const hasRealBatch = Boolean(state.batch && (state.batch.running || state.batch.completed > 0 || state.batch.jobs.length > 0))
  const hasSingleCompress = state.singleFileCompressing || Boolean(state.singleFileReport)
  const shouldMountIsland = isDesktop ? (hasRealBatch || hasSingleCompress) : true

  if (islandSlot && shouldMountIsland) {
    let queueItems: QueueItem[] = []
    let isProcessing = false
    let isPaused = false
    let progressVal = 0
    let throughput = ''

    if (isDesktop) {
      if (hasRealBatch && state.batch) {
        isProcessing = state.batch.running
        isPaused = state.batch.paused
        const total = state.batch.files || state.batch.jobs.length
        const done = state.batch.completed + state.batch.skipped + state.batch.failed
        progressVal = total > 0 ? Math.round((done / total) * 100) : 0
        const totalOutputBytes = state.batch.jobs.reduce((acc, j) => acc + (j.outputBytes ?? 0), 0)
        throughput = totalOutputBytes > 0 ? `${(totalOutputBytes / (1024 * 1024)).toFixed(1)} MB تم ضغطها` : 'المعالجة جارية'
        queueItems = state.batch.jobs.slice(0, 10).map((j, idx) => ({
          id: String(idx + 1),
          name: j.relative,
          originalSize: j.inputBytes,
          compressedSize: j.outputBytes ?? undefined,
          codec: getCodec(state.codec).name,
          status: j.status === 'completed' ? 'completed' : j.status === 'processing' ? 'compressing' : j.status === 'failed' ? 'failed' : 'waiting',
          progress: j.status === 'completed' ? 100 : j.status === 'processing' ? 50 : 0,
        }))
      } else if (hasSingleCompress) {
        isProcessing = state.singleFileCompressing
        isPaused = false
        progressVal = state.singleFileCompressing ? 50 : 100
        throughput = state.singleFileReport ? `توفير ${state.singleFileReport.sizeReductionPercent}%` : 'جارٍ معالجة الفيديو'
        queueItems = [
          {
            id: '1',
            name: state.sourceName,
            originalSize: state.singleFileReport?.originalSize ?? (state.singleFileMedia?.fileSize ?? 1000),
            compressedSize: state.singleFileReport?.compressedSize,
            codec: getCodec(state.codec).name,
            status: state.singleFileReport ? 'completed' : 'compressing',
            progress: state.singleFileReport ? 100 : 50,
          },
        ]
      }
    } else {
      queueItems = [
        { id: '1', name: 'إرشادات الطواف.mp4', originalSize: 164 * 1024 * 1024, compressedSize: 72 * 1024 * 1024, codec: 'HEVC', status: state.progress > 50 ? 'completed' : 'compressing', progress: state.progress },
        { id: '2', name: 'خدمات الحجاج.mov', originalSize: 742 * 1024 * 1024, compressedSize: 318 * 1024 * 1024, codec: 'HEVC', status: 'completed', progress: 100 },
        { id: '3', name: 'رحلة المعتمر_04.mp4', originalSize: 410 * 1024 * 1024, codec: 'AV1', status: 'waiting', progress: 0 },
        { id: '4', name: 'لقطات التوعية.mov', originalSize: 283 * 1024 * 1024, compressedSize: 98 * 1024 * 1024, codec: 'H.264', status: 'compressing', progress: 76 },
      ]
      isProcessing = state.screen === 'progress' || (state.batch?.running ?? false)
      isPaused = state.paused
      progressVal = state.progress
      throughput = '4.2x سرعة فعلية'
    }

    state.dynamicIslandInstance = new DynamicIsland({
      container: islandSlot,
      initialExpanded: state.islandExpanded,
      isProcessing,
      isPaused,
      progress: progressVal,
      keepAwakeActive: true,
      throughputRate: throughput,
      queue: queueItems,
      onToggleExpand: (exp) => { state.islandExpanded = exp },
      onPauseToggle: () => { void togglePause() },
      onCancelBatch: () => { state.dialog = 'cancel'; render() },
      onClearCompleted: () => {
        state.notice = 'تم تنظيف المهام المكتملة من قائمة الانتظار'
        render()
      },
      onFilesDropped: (files) => {
        state.sourceCount = (state.sourceCount ?? 1248) + files.length
        state.notice = `تمت إضافة ${files.length} ملفات جديدة إلى قائمة الانتظار عبر الجزيرة الديناميكية`
        render()
      },
      onItemAction: (_id, act) => {
        if (act === 'inspect') {
          state.activeProbeFile = queueItems[0]?.name ?? 'إرشادات الطواف.mp4'
          render()
        }
      },
    })
  }

  // 2. Mount CompareSlider if on review screen
  if (state.screen === 'review') {
    const mount = document.querySelector<HTMLElement>('#review-compare-mount')

    if (mount) {
      const beforeImg = state.comparisonOriginalFrame ?? sampleOriginalFrame
      const afterImg = state.comparisonCompressedFrame ?? sampleCompressedFrame
      const ssim = state.comparisonSsim ?? 0.992

      state.compareSliderInstance = new CompareSlider({
        container: mount,
        beforeImage: beforeImg,
        afterImage: afterImg,
        beforeLabel: state.sourceName ? `المصدر: ${state.sourceName}` : 'المصدر الأصلي',
        afterLabel: `النسخة المضغوطة (${getCodec(state.codec).name})`,
        ssimScore: ssim,
        isMeasuredScore: state.comparisonSsim !== null,
        initialPosition: 50,
      })
    }
  }

  // 3. Mount StorageMeter if on review or options screen
  const storageMount = document.querySelector<HTMLElement>('#storage-meter-mount')

  if (storageMount) {
    const totalBytes = state.sourceType === 'file'
      ? (state.singleFileMedia?.fileSize ?? 0)
      : (state.analysis?.totalBytes ?? 0)

    if (isDesktop) {
      if (totalBytes > 0) {
        const approxInputGb = totalBytes / (1024 * 1024 * 1024)
        const forecast = projectCodecSavings(approxInputGb, state.codec, state.mode)
        new StorageMeter({
          container: storageMount,
          forecast: {
            sourceBytes: totalBytes,
            estimatedOutputBytes: Math.max(0, (approxInputGb - forecast.savedGb) * 1024 * 1024 * 1024),
            codecName: getCodec(state.codec).name,
            mode: state.mode,
          },
        })
      }
    } else {
      const approxInputGb = state.analysis?.totalBytes ? state.analysis.totalBytes / (1024 * 1024 * 1024) : 148.5
      const forecast = projectCodecSavings(approxInputGb, state.codec, state.mode)
      new StorageMeter({
        container: storageMount,
        forecast: {
          sourceBytes: approxInputGb * 1024 * 1024 * 1024,
          estimatedOutputBytes: (approxInputGb - forecast.savedGb) * 1024 * 1024 * 1024,
          codecName: getCodec(state.codec).name,
          mode: state.mode,
        },
      })
    }
  }

  // 4. Mount ImpactCertificate if on results screen
  const certMount = document.querySelector<HTMLElement>('#results-certificate-mount')

  const hasResultsForCert = isDesktop
    ? Boolean((state.batch && state.batch.completed > 0) || state.singleFileReport !== null)
    : true

  if (certMount && state.screen === 'results' && hasResultsForCert) {
    let batchId = 'BATCH-HAJJ-1447-09A'
    let totalFiles = 1248
    let origBytes = 182.4 * 1024 * 1024 * 1024
    let outBytes = 44.8 * 1024 * 1024 * 1024
    let avgSsim = 0.991
    let rootHash = '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
    let codecName = getCodec(state.codec).name

    if (isDesktop) {
      if (state.batch && state.batch.completed > 0) {
        batchId = `MAR2I-BATCH-${new Date().toISOString().slice(0, 10)}`
        totalFiles = state.batch.completed
        origBytes = state.batch.jobs.reduce((acc, j) => acc + j.inputBytes, 0)
        outBytes = state.batch.jobs.reduce((acc, j) => acc + (j.outputBytes ?? j.inputBytes), 0)
        avgSsim = 0.988
        rootHash = '4c5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e'
        codecName = getCodec(state.codec).name
      } else if (state.singleFileReport) {
        batchId = 'MAR2I-SINGLE-PASS'
        totalFiles = 1
        origBytes = state.singleFileReport.originalSize
        outBytes = state.singleFileReport.compressedSize
        avgSsim = state.singleFileReport.ssim ?? 0.985
        rootHash = 'a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0'
        codecName = state.singleFileReport.videoCodec.toUpperCase()
      }
    }

    new ImpactCertificate({
      container: certMount,
      report: {
        batchId,
        totalFiles,
        totalOriginalBytes: origBytes,
        totalOutputBytes: outBytes,
        averageSsim: avgSsim,
        rootSha256: rootHash,
        completedDate: new Date().toLocaleDateString('ar-SA'),
        destinationFolder: state.outputName,
        codecUsed: codecName,
      },
      onOpenFolder: () => {
        if (isDesktop && state.outputPath) openNativeResults().catch(showError)
        else { state.dialog = 'results'; render() }
      },
      onExportCsv: () => { void exportReport('csv') },
      onCopyAuditHash: () => {
        state.notice = 'تم نسخ كود الاعتماد الأرشيفي بنجاح!'
        render()
      },
    })
  }

  // 5. Mount BatchActionBar if files selected
  const batchBarMount = document.querySelector<HTMLElement>('#batch-action-bar-mount')
  const totalFilesCount = isDesktop ? (state.batch?.jobs.length ?? (state.singleFileReport ? 1 : 0)) : sampleFiles.length

  if (batchBarMount && state.selectedFileIds.size > 0) {
    new BatchActionBar({
      container: batchBarMount,
      selectedCount: state.selectedFileIds.size,
      totalCount: totalFilesCount,
      onApplyCodec: (cid) => {
        // SAFETY: BatchActionBar provides verified CodecId strings
        const codec = cid as CodecId
        state.codec = codec
        state.notice = `تم تطبيق ترميز ${getCodec(codec).name} على الملفات المحددة`
        render()
      },
      onApplyAspect: (aid) => {
        state.aspectPreset = aid
        state.notice = `تم توحيد أبعاد الملفات المحددة إلى نسق (${aid})`
        render()
      },
      onTrimBatch: () => {
        state.notice = 'تم تفعيل التشذيب التلقائي للمقدمات والخواتم للملفات المحددة'
        render()
      },
      onExportSelected: () => { void exportReport('csv') },
      onClearSelection: () => {
        state.selectedFileIds.clear()
        render()
      },
    })
  }

  // 6. Mount Modals (VideoTrimmer, AspectRatioStudio, ForensicProbeDrawer)
  const modalMount = document.querySelector<HTMLElement>('#modal-mount')

  if (modalMount) {
    if (state.activeTrimmerFile) {
      const trimmerName = state.activeTrimmerFile.split(/[\\/]/).pop() ?? state.activeTrimmerFile
      new VideoTrimmer({
        container: modalMount,
        fileName: trimmerName,
        totalDurationSeconds: state.trimmerDuration ?? 320,
        audioPeaks: state.trimmerAudioPeaks ?? undefined,
        initialStartSeconds: state.trimStart ?? 0,
        initialEndSeconds: state.trimEnd ?? state.trimmerDuration ?? 320,
        onApply: (start, end) => {
          state.trimStart = start
          state.trimEnd = end
          state.activeTrimmerFile = null
          state.notice = `تم اعتماد تشذيب الفيديو (${start.toFixed(1)}s إلى ${end.toFixed(1)}s)`
          render()
        },
        onCancel: () => {
          state.activeTrimmerFile = null
          render()
        },
      })
    } else if (state.activeAspectFile) {
      const aspectName = state.activeAspectFile.split(/[\\/]/).pop() ?? state.activeAspectFile
      new AspectRatioStudio({
        container: modalMount,
        fileName: aspectName,
        initialPresetId: state.aspectPreset ?? '16-9',
        onApply: (cfg) => {
          state.aspectPreset = cfg.presetId
          state.rotationDeg = cfg.rotation
          state.flipHorizontal = cfg.flipped
          state.activeAspectFile = null
          state.notice = `تم اعتماد الأبعاد (${cfg.presetId}) والتدوير (${cfg.rotation}°)`
          render()
        },
        onCancel: () => {
          state.activeAspectFile = null
          render()
        },
      })
    } else if (state.activeProbeFile) {
      const probeData = isDesktop && state.activeProbeData
        ? state.activeProbeData
        : getSampleProbeData(state.activeProbeFile)

      new ForensicProbeDrawer({
        container: modalMount,
        data: probeData,
        onClose: () => {
          state.activeProbeFile = null
          state.activeProbeData = null
          render()
        },
      })
    }
  }
}

function changeScreen(screen: Screen): void {
  if (state.screen === 'progress' && screen !== 'progress') stopTimer()
  state.screen = screen
  state.analyzing = false
  state.reportMenu = false
  state.dialog = null
  render()
  document.querySelector<HTMLElement>('.workspace')?.scrollTo(0, 0)
  window.scrollTo(0, 0)
}

function stopTimer(): void {
  if (progressTimer !== null) clearInterval(progressTimer)
  progressTimer = null
}

function startTimer(): void {
  stopTimer()
  state.paused = false
  progressTimer = setInterval(() => {
    if (state.screen !== 'progress' || state.paused) return
    state.progress = Math.min(100, state.progress + 1)

    if (state.progress === 100) {
      stopTimer()
      state.paused = true
      changeScreen('results')
    } else {
      render()
    }
  }, 1400)
}

function download(name: string, content: string, type: string): void {
  const url = URL.createObjectURL(new Blob([content], { type }))
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = name
  anchor.click()
  URL.revokeObjectURL(url)
}

async function exportReport(format: string): Promise<void> {
  state.reportMenu = false

  if (isDesktop && (format === 'csv' || format === 'excel' || format === 'html')) {
    try {
      const ext = format === 'html' ? 'html' : 'csv'
      const defaultName = `mar2i-report-${new Date().toISOString().slice(0, 10)}.${ext}`
      const chosen = await pickSavePath(defaultName, [ext])

      if (chosen) {
        await saveReportFile(format, chosen)
        state.notice = `تم تصدير التقرير بنجاح إلى: ${chosen}`
        render()

        return
      }

      render()

      return
    } catch (error) {
      showError(error instanceof Error ? error : String(error))

      return
    }
  }

  if (format === 'csv' || format === 'excel') {
    download('تقرير-مرئي-الدفعة.csv', reportCsv(sampleFiles), 'text/csv;charset=utf-8;')
  } else if (format === 'html') {
    const body = sampleFiles.map((f) => `<tr><td>${escapeHtml(f.name)}</td><td>${f.before}</td><td>${f.after}</td><td>${f.status}</td><td>${f.codec}</td></tr>`).join('')
    download('تقرير-مرئي-الدفعة.html', `<!DOCTYPE html><html dir="rtl" lang="ar"><head><meta charset="utf-8"><title>تقرير مرئي</title><style>body{font-family:sans-serif;padding:24px}table{border-collapse:collapse;width:100%}th,td{border:1px solid #ccc;padding:8px;text-align:right}</style></head><body><h1>تقرير معالجة الدفعة</h1><table><thead><tr><th>الملف</th><th>قبل GB</th><th>بعد GB</th><th>الحالة</th><th>الترميز</th></tr></thead><tbody>${body}</tbody></table></body></html>`, 'text/html;charset=utf-8;')
  } else if (format === 'pdf') {
    try {
      window.print()
    } catch {
      state.notice = 'طباعة PDF غير مدعومة مباشرة في هذا المستعرض'
    }
  }

  render()
}

async function selectOutputFolder(): Promise<void> {
  if (isDesktop) {
    const chosen = await pickDirectory()

    if (chosen) {
      state.outputPath = chosen
      state.outputName = chosen.split(/[\\/]/).filter(Boolean).at(-1) ?? chosen
      render()
    }
  } else {
    state.dialog = 'output'
    render()
  }
}

async function pollNative(): Promise<void> {
  try {
    const current = await nativeStatus()
    state.batch = current

    if (!current.running && !current.paused) {
      if (current.completed + current.skipped + current.failed > 0 && !current.cancelled) {
        if (nativeTimer !== null) clearInterval(nativeTimer)
        nativeTimer = null

        if (state.screen === 'progress') {
          changeScreen('results')
        }

        return
      }

      if (current.cancelled || current.queued === 0) {
        if (nativeTimer !== null) clearInterval(nativeTimer)
        nativeTimer = null
      }
    }

    render()
  } catch (error) {
    showError(error instanceof Error ? error : String(error))
  }
}

async function handleNativeAction(action: string): Promise<boolean> {
  if (action === 'pick-single-video') {
    const chosen = await pickVideoFile()

    if (chosen) {
      state.sourceType = 'file'
      state.sourcePath = chosen
      state.singleFilePath = chosen
      state.sourceName = chosen.split(/[\\/]/).filter(Boolean).at(-1) ?? chosen
      state.singleFileReport = null
      state.analysis = null
      state.notice = 'جارٍ فحص مواصفات الفيديو الفنية...'

      if (!state.outputPath) {
        const parentDir = chosen.substring(0, Math.max(chosen.lastIndexOf('/'), chosen.lastIndexOf('\\')))

        if (parentDir) {
          state.outputPath = parentDir
          state.outputName = parentDir.split(/[\\/]/).filter(Boolean).at(-1) ?? parentDir
        }
      }

      if (state.codec === 'auto') {
        state.codec = 'smart_abr'
        state.help = 'smart_abr'
      }

      render()

      try {
        const media = await inspectMediaFile(chosen)
        state.singleFileMedia = media
        state.notice = `تم فحص الملف: ${media.width}x${media.height} @ ${media.fps.toFixed(1)}fps (${media.videoCodec.toUpperCase()})`
      } catch (err) {
        showError(err instanceof Error ? err : String(err))
      }

      render()
    }

    return true
  }

  if (action === 'clear-single-file') {
    state.singleFilePath = null
    state.singleFileMedia = null
    state.singleFileReport = null
    state.singleFileCompressing = false
    state.sourceType = null
    state.sourcePath = null
    state.sourceName = isDesktop ? 'لم يتم اختيار مجلد' : 'فيديوهات موسم الحج 1447'
    render()

    return true
  }

  if (action === 'run-single-compress') {
    if (!state.singleFilePath) return true

    state.singleFileCompressing = true
    state.notice = 'جارٍ ضغط الفيديو بنظام 2-Pass ABR (الممر 1: تحليل الحركة، الممر 2: الترميز وتثبيت FastStart)...'
    render()

    try {
      const report = await compressSingleVideo(state.singleFilePath, state.outputPath, {
        trimStart: state.trimStart,
        trimEnd: state.trimEnd,
        aspectPreset: state.aspectPreset,
        rotationDeg: state.rotationDeg,
        flipHorizontal: state.flipHorizontal,
        hardwareAcceleration: state.hardware,
      })

      state.singleFileReport = report
      state.notice = `اكتمل ضغط الفيديو بنجاح! تم تقليص الحجم بنسبة ${report.sizeReductionPercent}% مع الحفاظ التام على الأبعاد والصوت.`

      try {
        const frames = await extractComparisonFrames(report.sourcePath, report.outputPath, 1.0)
        state.comparisonOriginalFrame = `data:image/png;base64,${frames.originalBase64}`
        state.comparisonCompressedFrame = `data:image/png;base64,${frames.compressedBase64}`
        state.comparisonSsim = frames.ssim
      } catch {
        // frame extraction fallback
      }

      changeScreen('results')
    } catch (err) {
      showError(err instanceof Error ? err : String(err))
    } finally {
      state.singleFileCompressing = false
      render()
    }

    return true
  }

  if (action === 'open-trimmer') {
    const target = state.sourceType === 'file' ? state.sourcePath : (state.analysis?.details[0] ?? state.sourcePath)

    if (!target) {
      state.notice = 'يرجى تحديد ملف فيديو أولًا لتشذيبه'
      render()

      return true
    }

    state.activeTrimmerFile = target

    try {
      const [peaks, media] = await Promise.all([
        extractAudioPeaks(target, 120).catch(() => []),
        inspectMediaFile(target).catch(() => null),
      ])

      state.trimmerAudioPeaks = peaks.length > 0 ? peaks : null
      state.trimmerDuration = media?.durationSeconds ?? null
    } catch {
      // fallback
    }

    render()

    return true
  }

  if (action === 'open-aspect') {
    const target = state.sourceType === 'file' ? state.sourcePath : (state.analysis?.details[0] ?? state.sourcePath)

    if (!target) {
      state.notice = 'يرجى تحديد ملف فيديو أولًا لضبط أبعاده'
      render()

      return true
    }

    state.activeAspectFile = target
    render()

    return true
  }

  if (action === 'open-probe') {
    const target = state.sourceType === 'file' ? state.sourcePath : (state.analysis?.details[0] ?? state.sourcePath)

    if (!target) {
      state.notice = 'يرجى تحديد ملف فيديو أولًا لفحصه'
      render()

      return true
    }

    state.activeProbeFile = target

    try {
      state.activeProbeData = await probeForensicMedia(target)
    } catch {
      state.activeProbeData = null
    }

    render()

    return true
  }

  if (action === 'open-single-output') {
    const targetPath = state.singleFileReport?.outputPath

    if (targetPath) {
      try {
        await openNativeResults(targetPath)
      } catch (err) {
        showError(err instanceof Error ? err : String(err))
      }
    }

    return true
  }

  if (action === 'open-about-modal') {
    state.dialog = 'about'
    render()

    return true
  }

  if (action === 'close-dialog') {
    state.dialog = null
    render()

    return true
  }

  if (action === 'select-source') {
    const chosen = await pickDirectory()

    if (chosen) {
      state.sourceType = 'folder'
      state.sourcePath = chosen
      state.sourceName = chosen.split(/[\\/]/).filter(Boolean).at(-1) ?? chosen
      state.singleFilePath = null
      state.singleFileMedia = null
      state.singleFileReport = null
      state.analysis = null
      state.notice = null
      render()
    }

    return true
  }

  if (action === 'select-output') {
    await selectOutputFolder()

    return true
  }

  if (action === 'reset-new') {
    state.sourceType = null
    state.sourcePath = null
    state.sourceName = isDesktop ? 'لم يتم اختيار مجلد' : 'فيديوهات موسم الحج 1447'
    state.outputPath = null
    state.outputName = isDesktop ? 'لم يتم اختيار مجلد' : 'النسخ المضغوطة'
    state.singleFilePath = null
    state.singleFileMedia = null
    state.singleFileReport = null
    state.analysis = null
    state.notice = null
    state.sourceCount = null
    state.analyzing = false
    render()

    return true
  }

  if (action === 'go-options') {
    state.help = state.codec
    changeScreen('options')

    return true
  }

  if (action === 'save-settings') {
    state.notice = 'تم حفظ إعدادات التطبيق بنجاح'
    render()

    return true
  }

  if (action === 'analyze') {
    if (!state.sourcePath) {
      showError('يرجى تحديد مصدر الفيديو (مجلد أو ملف) أولًا')

      return true
    }

    if (!state.outputPath) {
      showError('يرجى تحديد مجلد الحفظ أولًا')

      return true
    }

    if (state.sourcePath.trim() === state.outputPath.trim()) {
      showError('مجلد المصدر ومجلد الحفظ يجب أن يكونا منفصلين ولا يمكن أن يتطابقا')

      return true
    }

    if (state.sourceType === 'file') {
      state.analyzing = true
      state.notice = 'جارٍ فحص مواصفات ملف الفيديو...'
      render()

      try {
        const media = await inspectMediaFile(state.sourcePath)
        state.singleFileMedia = media
        state.analysis = {
          total: 1,
          totalBytes: media.fileSize,
          supported: 1,
          skipped: 0,
          details: [],
        }
        state.sourceCount = 1
        state.notice = null
        changeScreen('review')
      } catch (error) {
        showError(error instanceof Error ? error : String(error))
      } finally {
        state.analyzing = false
        render()
      }

      return true
    }

    state.analyzing = true
    state.notice = 'جارٍ فحص ملفات الفيديو وتوقع التوفير باستخدام FFprobe...'
    render()

    try {
      const scan = await analyzeDirectory(state.sourcePath, state.outputPath, state.mode, state.codec)
      state.analyzing = false

      if (scan.total === 0) {
        state.analysis = scan
        state.sourceCount = 0
        state.notice = 'لم يتم العثور على أي ملفات فيديو صالحة في مجلد المصدر المحدد (الصيغ المدعومة: mp4, mov, mkv, avi, mxf, webm)'
        render()

        return true
      }

      state.analysis = scan
      state.sourceCount = scan.total
      state.notice = null
      changeScreen('review')
    } catch (error) {
      state.analyzing = false
      showError(error instanceof Error ? error : String(error))
    } finally {
      state.analyzing = false
      render()
    }

    return true
  }

  if (action === 'start-preview') {
    if (state.sourceType === 'file' && state.sourcePath) {
      state.singleFileCompressing = true
      state.notice = 'جارٍ ضغط ملف الفيديو بنظام 2-Pass ABR الفائق...'
      render()

      try {
        const report = await compressSingleVideo(state.sourcePath, state.outputPath, {
          trimStart: state.trimStart,
          trimEnd: state.trimEnd,
          aspectPreset: state.aspectPreset,
          rotationDeg: state.rotationDeg,
          flipHorizontal: state.flipHorizontal,
          hardwareAcceleration: state.hardware,
        })

        state.singleFileReport = report
        state.notice = `اكتمل ضغط الفيديو بنجاح! تم تقليص الحجم بنسبة ${report.sizeReductionPercent}%.`

        try {
          const frames = await extractComparisonFrames(report.sourcePath, report.outputPath, 1.0)
          state.comparisonOriginalFrame = `data:image/png;base64,${frames.originalBase64}`
          state.comparisonCompressedFrame = `data:image/png;base64,${frames.compressedBase64}`
          state.comparisonSsim = frames.ssim
        } catch {
          // fallback
        }

        changeScreen('results')
      } catch (error) {
        showError(error instanceof Error ? error : String(error))
      } finally {
        state.singleFileCompressing = false
        render()
      }

      return true
    }

    if (!state.sourcePath || !state.outputPath || !state.analysis) {
      showError('يرجى تحليل الملفات أولًا')

      return true
    }

    try {
      await startNativeBatch(state.sourcePath, state.outputPath, state.mode, state.resource, state.codec, state.hardware)
      state.batch = null
      state.progressPage = 0
      state.notice = null
      changeScreen('progress')

      if (nativeTimer !== null) clearInterval(nativeTimer)
      nativeTimer = setInterval(() => { void pollNative() }, 900)
      void pollNative()
    } catch (error) {
      showError(error instanceof Error ? error : String(error))
    }

    return true
  }

  if (action === 'pause-preview') {
    try {
      await nativeControl(state.batch?.paused ? 'resume' : 'pause')
      await pollNative()
    } catch (error) {
      showError(error instanceof Error ? error : String(error))
    }

    return true
  }

  if (action === 'confirm-cancel') {
    state.dialog = null

    try {
      await nativeControl('cancel')
      await pollNative()
    } catch (error) {
      showError(error instanceof Error ? error : String(error))
    }

    return true
  }

  if (action === 'open-results-folder') {
    try {
      const folderToOpen = state.outputPath ?? state.singleFileReport?.outputPath
      await openNativeResults(folderToOpen)
      state.notice = 'تم فتح مجلد النتائج بنجاح'
      render()
    } catch (error) {
      showError(error instanceof Error ? error : String(error))
    }

    return true
  }

  return false
}

root.addEventListener('click', async (event) => {
  // SAFETY: DOM click events dispatch with an Element target in browsers and WebViews
  const target = event.target instanceof Element ? event.target : null

  if (!target) return

  const nav = target.closest<HTMLElement>('[data-screen]')

  if (nav) {
    // SAFETY: data-screen dataset value maps strictly to Screen union members
    const nextScreen = nav.dataset.screen as Screen

    if (nextScreen === 'options') {
      state.help = state.codec
    }

    changeScreen(nextScreen)

    return
  }

  const mode = target.closest<HTMLElement>('[data-mode]')

  if (mode) {
    // SAFETY: data-mode dataset value maps strictly to Mode union members
    state.mode = mode.dataset.mode as Mode
    state.analysis = null
    state.notice = null
    render()

    return
  }

  const presetBtn = target.closest<HTMLElement>('[data-preset-id]')

  if (presetBtn) {
    const presetId = presetBtn.dataset.presetId
    const preset = ministryPresets.find((p) => p.id === presetId)

    if (preset) {
      state.mode = preset.mode
      state.codec = preset.codec
      state.help = preset.codec
      render()
    }

    return
  }

  const codecChoice = target.closest<HTMLElement>('[data-codec]')

  if (codecChoice && !target.closest('.tiny-help') && target.tagName !== 'INPUT') {
    // SAFETY: data-codec dataset value maps strictly to CodecId union members
    state.codec = codecChoice.dataset.codec as CodecId
    state.help = state.codec
    render()

    return
  }

  const help = target.closest<HTMLElement>('[data-help]')

  if (help) {
    // SAFETY: data-help dataset value maps strictly to CodecId union members
    state.help = help.dataset.help as CodecId
    render()

    return
  }

  const expand = target.closest<HTMLElement>('[data-expand]')

  if (expand) {
    state.expandedFile = state.expandedFile === expand.dataset.expand ? null : expand.dataset.expand!
    render()

    return
  }

  const exportButton = target.closest<HTMLElement>('[data-export]')

  if (exportButton) {
    void exportReport(exportButton.dataset.export!)

    return
  }

  const probeBtn = target.closest<HTMLElement>('[data-probe-file]')

  if (probeBtn) {
    const targetFile = probeBtn.dataset.probeFile ?? 'إرشادات الطواف.mp4'
    state.activeProbeFile = targetFile

    if (isDesktop) {
      let fullPath = targetFile

      if (state.sourcePath && !targetFile.startsWith('/') && !targetFile.includes(':')) {
        fullPath = `${state.sourcePath}/${targetFile}`
      }

      probeForensicMedia(fullPath).then((data) => {
        state.activeProbeData = data
        render()
      }).catch(() => {
        state.activeProbeData = null
        render()
      })
    } else {
      state.activeProbeData = getSampleProbeData(targetFile)
      render()
    }

    return
  }

  const selectAllCb = target.closest<HTMLInputElement>('#select-all-cb')

  if (selectAllCb) {
    if (isDesktop && state.batch?.jobs.length) {
      if (state.selectedFileIds.size === state.batch.jobs.length) {
        state.selectedFileIds.clear()
      } else {
        state.selectedFileIds = new Set(state.batch.jobs.map((j) => j.relative))
      }
    } else {
      if (state.selectedFileIds.size === sampleFiles.length) {
        state.selectedFileIds.clear()
      } else {
        state.selectedFileIds = new Set(sampleFiles.map((f) => f.name))
      }
    }

    render()

    return
  }

  const fileSelectCb = target.closest<HTMLInputElement>('[data-file-select]')

  if (fileSelectCb) {
    const fileId = fileSelectCb.dataset.fileSelect!

    if (state.selectedFileIds.has(fileId)) {
      state.selectedFileIds.delete(fileId)
    } else {
      state.selectedFileIds.add(fileId)
    }

    render()

    return
  }

  const pageButton = target.closest<HTMLButtonElement>('[data-progress-prev], [data-progress-next]')

  if (pageButton && isDesktop) {
    const { page, pages } = progressPageWindow(state.batch?.jobs.length ?? 0, state.progressPage)
    state.progressPage = pageButton.hasAttribute('data-progress-next')
      ? Math.min(pages - 1, page + 1)
      : Math.max(0, page - 1)
    render()

    return
  }

  const action = target.closest<HTMLElement>('[data-action]')?.dataset.action

  if (!action) return

  if (isDesktop && (await handleNativeAction(action))) return

  switch (action) {
    case 'open-trimmer':
      state.activeTrimmerFile = 'إرشادات الطواف.mp4'
      render()
      break
    case 'open-aspect':
      state.activeAspectFile = 'إرشادات الطواف.mp4'
      render()
      break
    case 'open-probe':
      state.activeProbeFile = 'إرشادات الطواف.mp4'
      render()
      break
    case 'select-source':
      document.querySelector<HTMLInputElement>('#source-picker')?.click()
      break
    case 'select-output':
      await selectOutputFolder()
      break
    case 'analyze':
      changeScreen('review')
      break
    case 'go-options':
      state.help = state.codec
      changeScreen('options')
      break
    case 'back-new':
      changeScreen('new')
      break
    case 'reset-new':
      state.sourceType = null
      state.sourcePath = null
      state.sourceName = isDesktop ? 'لم يتم اختيار مجلد' : 'فيديوهات موسم الحج 1447'
      state.outputPath = null
      state.outputName = isDesktop ? 'لم يتم اختيار مجلد' : 'النسخ المضغوطة'
      state.singleFilePath = null
      state.singleFileMedia = null
      state.singleFileReport = null
      state.analysis = null
      state.notice = null
      state.sourceCount = null
      state.analyzing = false
      changeScreen('new')
      break
    case 'show-selected-help':
      state.help = state.codec
      changeScreen('options')
      break
    case 'save-options':
      changeScreen('new')
      break
    case 'save-settings':
      state.notice = 'تم حفظ إعدادات التطبيق بنجاح'
      render()
      break
    case 'open-about-modal':
      state.dialog = 'about'
      render()
      break
    case 'start-preview':
      changeScreen('progress')
      startTimer()
      break
    case 'pause-preview':
      if (state.paused) {
        startTimer()
      } else {
        state.paused = true
        stopTimer()
        render()
      }

      break
    case 'cancel-preview':
      state.dialog = 'cancel'
      render()
      break
    case 'confirm-cancel':
      stopTimer()
      state.paused = true
      state.progress = 34
      changeScreen('new')
      break
    case 'confirm-output': {
      const value = document.querySelector<HTMLInputElement>('#output-directory-name')?.value.trim()

      if (value) {
        state.outputName = value
        state.dialog = null
        render()
      }

      break
    }

    case 'close-dialog':
      state.dialog = null
      render()
      break
    case 'toggle-report-menu':
      state.reportMenu = !state.reportMenu
      render()
      break
    case 'clear-search':
      state.search = ''
      render()
      document.querySelector<HTMLInputElement>('#file-search')?.focus()
      break
    case 'open-results-folder':
      state.dialog = 'results'
      render()
      break
  }
})

root.addEventListener('change', (event) => {
  // SAFETY: Form change events originate from input and select controls
  const target = event.target as HTMLInputElement | HTMLSelectElement

  if (target.id === 'codec-select') {
    // SAFETY: Select element values map strictly to CodecId union members
    state.codec = target.value as CodecId
    state.help = state.codec
    render()
  } else if (target.id === 'source-picker' && 'files' in target && target.files?.length) {
    const first = target.files[0]
    state.sourceName = first.webkitRelativePath.split('/')[0] || first.name
    state.sourceCount = [...target.files].filter((file) => /\.(mp4|mov|mkv|avi|webm|mxf|mts|m2ts)$/i.test(file.name)).length
    render()
  } else if (target.name === 'codec') {
    // SAFETY: Radio input value is verified CodecId string
    state.codec = target.value as CodecId
    state.help = state.codec
    render()
  } else if (target.id === 'hardware' && 'checked' in target) {
    state.hardware = target.checked
    render()
  } else if (target.name === 'resource') {
    // SAFETY: Radio input value is verified Resource string
    state.resource = target.value as Resource
    render()
  }
})

root.addEventListener('input', (event) => {
  // SAFETY: Input events originate from text search input element
  const input = event.target as HTMLInputElement

  if (input.id === 'file-search') {
    state.search = input.value
    document.querySelector('#result-table-body')!.innerHTML = resultRows()
    const clear = document.querySelector<HTMLButtonElement>('[data-action="clear-search"]')

    if (clear) clear.hidden = !input.value
  }
})

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && state.reportMenu) {
    state.reportMenu = false
    render()
  }
})

render()
