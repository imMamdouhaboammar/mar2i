import './style.css'
import { isDesktop, pickDirectory, analyzeDirectory, startNativeBatch, nativeStatus, nativeControl, openNativeResults, nativeReportContent, type ScanResult, type BatchStatus } from './native'
import { codecs, escapeHtml, getCodec, reportCsv, sampleFiles, savingsPercent, type CodecId, type Mode, type Resource, type Screen } from './data'

type FolderDialog = 'output' | 'cancel' | 'results' | null

type UiState = {
  screen: Screen
  mode: Mode
  codec: CodecId
  help: CodecId
  resource: Resource
  hardware: boolean
  sourceName: string
  outputName: string
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
  paused: boolean
  dialog: FolderDialog
}

const state: UiState = {
  screen: 'new', mode: 'publish', codec: 'auto', help: 'av1', resource: 'balanced', hardware: true,
  sourceName: isDesktop ? 'لم يتم اختيار مجلد' : 'فيديوهات موسم الحج 1447', outputName: isDesktop ? 'لم يتم اختيار مجلد' : 'النسخ المضغوطة', sourceCount: null,
  sourcePath: null, outputPath: null, analysis: null, batch: null, notice: null,
  search: '', reportMenu: false, expandedFile: 'تسجيل_14.mov', progress: 34, paused: true, dialog: null,
}

const root = document.querySelector<HTMLDivElement>('#app')!
let progressTimer: ReturnType<typeof setInterval> | null = null
let nativeTimer: ReturnType<typeof setInterval> | null = null

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return 'غير متاح'
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let number = bytes / 1024
  let position = 0
  while (number >= 1024 && position < units.length - 1) { number /= 1024; position++ }
  return `${number.toFixed(number < 10 ? 1 : 0)} ${units[position]}`
}

function basename(path: string): string { return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path }
function showError(error: unknown): void { state.notice = String(error); render() }
const paths: Record<string, string> = {
  layers: '<path d="m12 2 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 17l9 5 9-5"/>',
  play: '<circle cx="12" cy="12" r="9"/><path d="m10 8 6 4-6 4V8Z"/>',
  report: '<rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 9h8M8 13h8M8 17h5"/>',
  settings: '<path d="M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Z"/><path d="M4.7 9.5 3 8l2-3 2.2.8a8 8 0 0 1 2.2-1.2L10 2h4l.6 2.6a8 8 0 0 1 2.2 1.2L19 5l2 3-1.7 1.5a8 8 0 0 1 0 5L21 16l-2 3-2.2-.8a8 8 0 0 1-2.2 1.2L14 22h-4l-.6-2.6a8 8 0 0 1-2.2-1.2L5 19l-2-3 1.7-1.5a8 8 0 0 1 0-5Z"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>',
  help: '<circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 2-2.5 2-2.5 4"/><path d="M12 17h.01"/>',
  chevron: '<path d="m6 9 6 6 6-6"/>',
  arrow: '<path d="m14 7-5 5 5 5"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  pause: '<path d="M9 5v14M15 5v14"/>',
  search: '<circle cx="10.8" cy="10.8" r="6.8"/><path d="m16 16 5 5"/>',
  download: '<path d="M12 3v12m0 0-4-4m4 4 4-4M4 17v3h16v-3"/>',
  alert: '<path d="M12 3 2 21h20L12 3Z"/><path d="M12 9v5m0 3h.01"/>',
  video: '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m10 9 5 3-5 3V9Z"/>',
  close: '<path d="m5 5 14 14M19 5 5 19"/>',
  switch: '<path d="M3 7h18M3 12h18M3 17h18"/>',
  back: '<path d="m15 18-6-6 6-6"/>',
}

function icon(name: string, size = 19): string {
  return `<svg class="icon" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] ?? paths.video}</svg>`
}

function button(label: string, action: string, variant = 'secondary', extra = ''): string {
  return `<button type="button" class="btn btn-${variant}" data-action="${action}" ${extra}>${label}</button>`
}

function navItem(screen: Screen, label: string, image: string): string {
  const active = state.screen === screen || (screen === 'new' && state.screen === 'review')
  return `<button type="button" class="nav-item${active ? ' active' : ''}" data-screen="${screen}" aria-current="${active ? 'page' : 'false'}">${icon(image, 20)}<span>${label}</span></button>`
}

function sidebar(): string {
  return `<aside class="sidebar" aria-label="التنقل الرئيسي">
    <div class="brand"><span class="brand-icon" aria-hidden="true"><i></i><i></i><i></i></span><span>مرئي</span></div>
    <nav class="sidebar-nav">
      ${navItem('new', 'الدفعات', 'layers')}
      ${navItem('progress', 'المعالجة', 'play')}
      ${navItem('results', 'التقارير', 'report')}
      ${navItem('settings', 'الإعدادات', 'settings')}
    </nav>
    <div class="sidebar-foot"><span class="availability-dot"></span> يعمل دون اتصال <span class="version">${isDesktop ? 'نسخة تطوير محلية' : 'واجهة تجريبية'}</span></div>
  </aside>`
}

function screenHeading(title: string, subtitle?: string, back = false): string {
  return `<header class="screen-heading">${back ? `<button class="back-link" type="button" data-action="back-new">${icon('back', 17)} العودة</button>` : ''}<h1>${title}</h1>${subtitle ? `<p>${subtitle}</p>` : ''}</header>`
}

function folderField(label: string, name: string, action: string, id: string): string {
  return `<div class="folder-row"><label for="${id}">${label}</label><div class="folder-value" id="${id}">${icon('folder', 21)}<span>${escapeHtml(name)}</span></div>${button('استعراض', action, 'outline')}</div>`
}

function modeChoices(): string {
  return `<div class="mode-grid" role="group" aria-label="الغرض من الملفات">
    <button type="button" data-mode="publish" class="mode-choice${state.mode === 'publish' ? ' selected' : ''}" aria-pressed="${state.mode === 'publish'}">
      <span class="radio-mark"></span><span><strong>النشر والمشاركة</strong><small>حجم أصغر وتوافق أفضل مع الأجهزة</small></span>
    </button>
    <button type="button" data-mode="archive" class="mode-choice${state.mode === 'archive' ? ' selected' : ''}" aria-pressed="${state.mode === 'archive'}">
      <span class="radio-mark"></span><span><strong>الأرشفة عالية الجودة</strong><small>حفظ أمين لنسخ الإنتاج</small></span>
    </button>
  </div>`
}

function renderNew(): string {
  return `<div class="page page-new">${screenHeading('دفعة جديدة', 'حدد ملفات الفيديو ووجهتها، ثم اختر طريقة المعالجة')}
    <section class="folder-section" aria-label="المجلدات">
      ${folderField('مجلد المصدر', state.sourceName, 'select-source', 'source-value')}
      ${folderField('مجلد الحفظ', state.outputName, 'select-output', 'output-value')}
      <input type="file" id="source-picker" webkitdirectory multiple hidden aria-label="اختيار مجلد المصدر" />
    </section>
    <section class="choice-section"><h2>الغرض من الملفات</h2>${modeChoices()}</section>
    <section class="codec-preview"><div class="field-title">اختيار الترميز</div><div class="select-surface">${getCodec(state.codec).name}${icon('chevron', 17)}</div>
      <button class="help-trigger" type="button" data-action="show-selected-help" aria-label="شرح اختيار الترميز">${icon('help', 22)}</button>
      <p>${state.codec === 'auto' ? 'يختار التطبيق الأنسب لكل فيديو بحسب الهدف والجودة المطلوبة' : getCodec(state.codec).description}</p>
    </section>
    <button class="text-action" type="button" data-screen="options">عرض جميع خيارات الضغط ${icon('arrow', 17)}</button>
    <footer class="page-actions">${button('تحليل الفيديوهات', 'analyze', 'primary')}${button('إلغاء', 'reset-new', 'outline')}</footer>
  </div>`
}

function codecRow(codec: typeof codecs[number]): string {
  const active = state.codec === codec.id
  return `<div class="codec-row${active ? ' selected' : ''}" role="row">
    <div class="codec-name" role="cell"><input type="radio" name="codec" value="${codec.id}" ${active ? 'checked' : ''} aria-label="اختيار ${codec.name}" /><strong dir="auto">${codec.name}</strong><button class="tiny-help${state.help === codec.id ? ' is-current' : ''}" data-help="${codec.id}" type="button" aria-label="شرح ${codec.name}">${icon('help', 18)}</button></div>
    <span role="cell">${codec.size}</span><span role="cell">${codec.speed}</span><span role="cell">${codec.compatibility}</span>
  </div>`
}

function toggle(): string {
  return `<label class="toggle-field"><input type="checkbox" id="hardware" ${state.hardware && !isDesktop ? 'checked' : ''} ${isDesktop ? 'disabled' : ''} /><span class="switch" aria-hidden="true"></span><span>${isDesktop ? 'تسريع العتاد غير متاح بعد في نسخة التطوير' : 'استخدام تسريع الجهاز عند توفره'}</span><span class="inline-help" title="قد يرفع السرعة، لكن جودة الضغط وحجمه يختلفان حسب الجهاز">${icon('help', 17)}</span></label>`
}

function renderOptions(): string {
  const selected = getCodec(state.help)
  const mismatch = state.mode === 'publish' && state.codec === 'ffv1'
  return `<div class="page page-options">${screenHeading('خيارات الضغط', 'اختر ما يناسب استخدام الفيديو، أو اترك الاختيار تلقائيًا', true)}
    <div class="segmented" role="group" aria-label="غرض المعالجة"><button class="${state.mode === 'publish' ? 'active' : ''}" data-mode="publish" aria-pressed="${state.mode === 'publish'}">النشر والمشاركة</button><button class="${state.mode === 'archive' ? 'active' : ''}" data-mode="archive" aria-pressed="${state.mode === 'archive'}">الأرشفة عالية الجودة</button></div>
    <div class="options-layout"><section class="codec-table" aria-label="مقارنة الترميزات" role="table">
      <div class="codec-row codec-header" role="row"><strong role="columnheader">الترميز</strong><strong role="columnheader">حجم الملف</strong><strong role="columnheader">سرعة المعالجة</strong><strong role="columnheader">التوافق</strong></div>
      ${codecs.map(codecRow).join('')}
    </section><aside class="codec-help" aria-live="polite"><div class="aside-top">${icon('help', 20)}<span>دليل الاختيار</span></div><h2>متى أختار <span dir="auto">${selected.name}</span>؟</h2><p>${selected.description}</p><div class="help-divider"></div><p class="caveat">${icon('alert', 18)}${selected.caveat}</p><div class="recommendation"><strong>التوصية</strong><p>${selected.recommendation}</p></div></aside></div>
    ${mismatch ? `<p class="inline-warning">${icon('alert', 18)} FFV1 مخصص للأرشفة عادةً، وقد لا يعمل على منصات نشر الفيديو</p>` : ''}
    ${isDesktop ? `<p class="inline-warning">${icon('help',18)} المعالجة الفعلية متاحة الآن بالاختيار التلقائي فقط. الخيارات الأخرى معروضة للمقارنة وسيتم تفعيلها بعد اختبارات الترميز والتحقق</p>` : ''}
    <div class="hardware-row">${toggle()}</div>
    <footer class="page-actions">${button('حفظ الخيارات', 'save-options', 'primary')}${button('رجوع', 'back-new', 'outline')}</footer>
  </div>`
}

function renderNativeReview() {
  const scan = state.analysis;
  const lines = (scan?.details ?? []).slice(0, 8).map(item => `<li>${escapeHtml(item)}</li>`).join('');
  return `<div class="page page-review">${screenHeading('مراجعة الدفعة','راجع الملفات المختارة قبل تشغيل FFmpeg محليًا',true)}
    <section class="review-summary"><h2>الملفات المحددة</h2>
      <div class="summary-line"><span>مجلد المصدر</span><strong>${escapeHtml(state.sourceName)}</strong></div>
      <div class="summary-line"><span>مجلد الحفظ</span><strong>${escapeHtml(state.outputName)}</strong></div>
      <div class="summary-line"><span>ملفات مرشحة</span><strong>${scan?.total ?? 0} فيديو</strong></div>
      <div class="summary-line"><span>صالحة لمسار المعالجة الحالي</span><strong>${scan?.supported ?? 0} ملف</strong></div>
      <div class="summary-line"><span>حجم المدخلات</span><strong dir="ltr">${formatBytes(scan?.totalBytes ?? 0)}</strong></div>
      <div class="summary-line"><span>الغرض</span><strong>${state.mode === 'publish' ? 'النشر والمشاركة' : 'الأرشفة عالية الجودة'}</strong></div>
      <div class="summary-line"><span>الترميز</span><strong>${getCodec(state.codec).name}</strong></div>
    </section>
    <div class="review-note">${icon('help',21)}<div><strong>معالجة حقيقية، مع شروط قبول صارمة</strong>
      <p>سيحتفظ التطبيق بكل الملفات الأصلية. بعض المصادر قد يتم نسخها دون تغيير أو رفضها عند تعذر ضمان الشروط المطلوبة</p>
      ${lines ? `<ul class="native-detail-list">${lines}</ul>` : ''}</div></div>
    <footer class="page-actions">${button('بدء المعالجة','start-preview','primary', scan?.total ? '' : 'disabled')}${button('تعديل الخيارات','go-options','outline')}</footer></div>`;
}

function renderReview(): string {
  if (isDesktop) return renderNativeReview()
  const total = state.sourceCount ?? 1248
  const title = 'مراجعة الدفعة'
  return `<div class="page page-review">${screenHeading(title, 'راجع الاختيارات قبل بدء المعالجة', true)}
    <section class="review-summary"><h2>الملفات المحددة</h2><div class="summary-line"><span>مجلد المصدر</span><strong>${escapeHtml(state.sourceName)}</strong></div><div class="summary-line"><span>مجلد الحفظ</span><strong>${escapeHtml(state.outputName)}</strong></div><div class="summary-line"><span>عدد الملفات</span><strong>${total.toLocaleString('en-US')} فيديو</strong></div><div class="summary-line"><span>الغرض</span><strong>${state.mode === 'publish' ? 'النشر والمشاركة' : 'الأرشفة عالية الجودة'}</strong></div><div class="summary-line"><span>الترميز</span><strong dir="auto">${getCodec(state.codec).name}</strong></div></section>
    <div class="review-note">${icon('help', 21)}<div><strong>هذه معاينة لتجربة الاستخدام</strong><p>التحليل والضغط في هذه النسخة توضيحيان، ولن تتغير أي ملفات على جهازك</p></div></div>
    <footer class="page-actions">${button('بدء المعالجة التجريبية', 'start-preview', 'primary')}${button('تعديل الخيارات', 'go-options', 'outline')}</footer>
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

function nativeProgressRows() {
  const jobs = state.batch?.jobs ?? [];
  const labels: Record<string,string> = { completed: 'مكتمل', skipped: 'تم التخطي', failed: 'فشل', processing: 'جارٍ الضغط', verifying: 'جارٍ التحقق', queued: 'بانتظار الدور', interrupted: 'بانتظار الاستكمال' };
  return jobs.slice(0, 30).map((job) => {
    const done = ['completed','skipped','failed'].includes(job.status);
    return `<tr><td class="name-cell">${icon('video', 18)}<span>${escapeHtml(job.relative)}</span></td>
      <td class="latin">${formatBytes(job.inputBytes)}</td><td class="latin">${job.outputBytes === null ? 'غير متاح' : formatBytes(job.outputBytes)}</td>
      <td><span class="status ${job.status === 'completed' ? 'complete' : job.status === 'failed' ? 'verifying' : 'waiting'}">${labels[job.status] ?? job.status}</span></td>
      <td><div class="cell-progress"><div class="mini-track"><span style="width:${done ? 100 : 0}%"></span></div><b class="latin">${done ? '100%' : 'قيد العمل'}</b></div></td></tr>`;
  }).join('') || '<tr><td colspan="5" class="no-results">جارٍ تجهيز قائمة الملفات</td></tr>';
}

function renderNativeProgress() {
  const batch = state.batch;
  const total = batch?.files ?? state.analysis?.total ?? 0;
  const completed = (batch?.completed ?? 0) + (batch?.skipped ?? 0) + (batch?.failed ?? 0);
  const percent = total ? Math.round(100 * completed / total) : 0;
  const paused = batch?.paused ?? false;
  return `<div class="page page-progress">${screenHeading('معالجة الدفعة', 'المساحة الموفرة هنا تخص الملفات التي اكتملت وتحققنا من قبولها فقط')}
    <section class="progress-panel"><div class="progress-main"><div class="progress-copy">
      <h2><span class="latin">${completed.toLocaleString('en-US')}</span> من <span class="latin">${total.toLocaleString('en-US')}</span> فيديو</h2>
      <div class="main-track" role="progressbar" aria-label="تقدم الدفعة" aria-valuenow="${percent}" aria-valuemin="0" aria-valuemax="100"><div style="width:${percent}%"></div></div>
      <span class="track-value latin">${percent}%</span></div>
      <div class="progress-buttons">${button(`${icon(paused ? 'play' : 'pause', 17)} ${paused ? 'استئناف' : 'إيقاف مؤقت'}`, 'pause-preview', 'primary', !batch?.running ? 'disabled' : '')}${button('إلغاء المهام المنتظرة', 'cancel-preview', 'outline', !batch?.running ? 'disabled' : '')}</div></div>
      <dl class="inline-metrics"><div><dt>قيد المعالجة</dt><dd>${batch?.active ?? 0} ملفات</dd></div><div><dt>بانتظار الدور</dt><dd>${batch?.queued ?? total} ملفات</dd></div><div><dt>التوفير المتحقق منه</dt><dd class="latin">${formatBytes(batch?.verifiedSavingsBytes ?? 0)}</dd></div></dl>
    </section><div class="section-title"><h2>حالة الملفات</h2><span>${batch?.running ? 'النتائج تتحدث أثناء العمل' : 'اكتملت العمليات المتاحة'}</span></div>
    <div class="table-scroll"><table class="data-table progress-table"><thead><tr><th>اسم الفيديو</th><th>الحجم الأصلي</th><th>الحجم الجديد</th><th>الحالة</th><th>النتيجة</th></tr></thead><tbody>${nativeProgressRows()}</tbody></table></div>
    <div class="review-note">${icon('help', 21)}<div><strong>معلومة عن الإيقاف والاستكمال</strong><p>الإيقاف المؤقت والإلغاء ينطبقان بين الملفات، ولا يستكمل التطبيق ترميز ملف من منتصفه بعد الانقطاع</p></div></div>
  </div>`;
}

function renderProgress(): string {
  if (isDesktop) return renderNativeProgress()
  const count = Math.round(1248 * state.progress / 100)
  return `<div class="page page-progress">${screenHeading('معالجة فيديوهات الحج 1447')}
    <section class="progress-panel"><div class="progress-main"><div class="progress-copy"><h2><span class="latin">${count.toLocaleString('en-US')}</span> من <span class="latin">1,248</span> فيديو</h2><div class="main-track" role="progressbar" aria-label="تقدم معالجة الملفات" aria-valuenow="${state.progress}" aria-valuemin="0" aria-valuemax="100"><div style="width:${state.progress}%"></div></div><span class="track-value latin">${state.progress}%</span></div><div class="progress-buttons">${button(`${icon(state.paused ? 'play' : 'pause', 17)} ${state.paused ? 'استئناف' : 'إيقاف مؤقت'}`, 'pause-preview', 'primary')}${button('إلغاء الدفعة', 'cancel-preview', 'outline')}</div></div>
      <dl class="inline-metrics"><div><dt>قيد المعالجة</dt><dd>3 ملفات</dd></div><div><dt>الوقت المتبقي</dt><dd>2 س 18 د</dd></div><div><dt>المساحة الموفرة</dt><dd class="latin">318.6 GB</dd></div></dl>
    </section><div class="section-title"><h2>الملفات الحالية</h2><span>معاينة لدفعة تجريبية</span></div>
    <div class="table-scroll"><table class="data-table progress-table"><thead><tr><th>اسم الفيديو</th><th>الحجم الأصلي</th><th>الحجم الجديد</th><th>الحالة</th><th>التقدم</th></tr></thead><tbody>${progressRows()}</tbody></table></div>
    <section class="workers"><h2>عمليات المعالجة</h2><div class="worker-grid"><div><strong>عملية 1</strong><span class="worker-active">نشطة</span><small>ضغط إرشادات الطواف.mp4</small></div><div><strong>عملية 2</strong><span class="worker-active">نشطة</span><small>ضغط لقطات التوعية.mov</small></div><div><strong>عملية 3</strong><span class="worker-active">نشطة</span><small>التحقق من الملف المكتمل</small></div></div></section>
  </div>`
}

function status(row: typeof sampleFiles[number]): string {
  if (row.status === 'complete') return `<span class="row-status successful">${icon('check', 15)} مكتمل</span>`
  if (row.status === 'unchanged') return '<span class="row-status neutral">دون تغيير</span>'
  return `<span class="row-status needs-review">${icon('alert', 16)} يحتاج مراجعة</span>`
}

function nativeResultRows() {
  const search = state.search.trim().toLocaleLowerCase('ar');
  const jobs = (state.batch?.jobs ?? []).filter(job => job.relative.toLocaleLowerCase('ar').includes(search));
  if (!jobs.length) return '<tr><td colspan="5" class="no-results">لا توجد ملفات تطابق البحث أو لم تبدأ أي دفعة بعد</td></tr>';
  return jobs.map(job => {
    const name = escapeHtml(job.relative);
    const saving = job.status === 'completed' && job.outputBytes !== null && job.inputBytes > 0 ? Math.max(0,100*(job.inputBytes-job.outputBytes)/job.inputBytes) : 0;
    const label = job.status === 'completed' ? 'مكتمل' : job.status === 'skipped' ? 'دون تغيير' : job.status === 'failed' ? 'فشل' : 'قيد العمل';
    return `<tr><td class="name-cell"><button class="row-toggle" data-expand="${name}" aria-expanded="${state.expandedFile === job.relative}">${icon('video',17)}<span>${name}</span>${icon('chevron',16)}</button></td>
      <td class="latin">${formatBytes(job.inputBytes)}</td><td class="latin">${job.outputBytes === null ? 'غير متاح' : formatBytes(job.outputBytes)}</td>
      <td class="latin">${saving.toFixed(1)}%</td><td><span class="row-status ${job.status === 'completed' ? 'successful' : job.status === 'failed' ? 'needs-review' : 'neutral'}">${label}</span></td></tr>
      ${state.expandedFile === job.relative ? `<tr class="detail-row"><td colspan="5"><div class="detail-note">${icon('help',18)}<div><strong>تفاصيل نتيجة الملف</strong><span>${escapeHtml(job.reason ?? job.quality ?? 'لم تتوفر تفاصيل إضافية')}</span></div></div></td></tr>` : ''}`;
  }).join('');
}

function renderNativeResults() {
  const status = state.batch;
  const completed = status?.completed ?? 0;
  const skipped = status?.skipped ?? 0;
  const failed = status?.failed ?? 0;
  return `<div class="page page-results">${screenHeading(status?.running ? 'النتائج حتى الآن' : 'نتائج الدفعة', 'المساحة الموفرة محسوبة من الملفات المقبولة فقط، والأصول لم تتغير')}
    <div class="results-overview"><div class="result-highlight"><strong class="latin">${formatBytes(status?.verifiedSavingsBytes ?? 0)}</strong><span>توفير مُتحقق منه</span></div>
      <dl class="results-counts"><div><dd class="latin">${completed}</dd><dt>مكتمل</dt></div><div><dd class="latin">${skipped}</dd><dt>دون ضغط</dt></div><div><dd class="latin">${failed}</dd><dt>فشل</dt></div></dl>
      <div class="result-actions">${button(`${icon('folder', 18)} فتح مجلد النتائج`,'open-results-folder','primary', !status ? 'disabled' : '')}
      <div class="menu-wrap">${button(`تصدير التقرير ${icon('chevron', 16)}`,'toggle-report-menu','outline', `aria-expanded="${state.reportMenu}" ${!status ? 'disabled' : ''}`)}
      ${state.reportMenu ? `<div class="export-menu" role="menu"><button role="menuitem" data-export="csv">CSV</button><button role="menuitem" data-export="excel">Excel (CSV)</button><button role="menuitem" data-export="html">HTML</button><button role="menuitem" data-export="pdf">PDF (طباعة)</button></div>` : ''}</div></div></div>
    <div class="section-title results-title"><h2>نتائج الملفات</h2><div class="search-box">${icon('search',19)}<input id="file-search" type="search" placeholder="البحث في الملفات" value="${escapeHtml(state.search)}" aria-label="البحث في الملفات"/><button type="button" data-action="clear-search" aria-label="مسح البحث" ${state.search ? '' : 'hidden'}>${icon('close',15)}</button></div></div>
    <div class="table-scroll"><table class="data-table results-table"><thead><tr><th>الملف</th><th>قبل</th><th>بعد</th><th>التوفير</th><th>الحالة</th></tr></thead><tbody id="result-table-body">${nativeResultRows()}</tbody></table></div>
    <p class="data-disclaimer">التقرير يعرض القيم المقاسة من عملية المعالجة الفعلية</p></div>`;
}

function resultRows(): string {
  if (isDesktop) return nativeResultRows()
  const query = state.search.trim().toLocaleLowerCase('ar')
  const filtered = sampleFiles.filter((file) => file.name.toLocaleLowerCase('ar').includes(query))
  if (!filtered.length) return `<tr><td colspan="5" class="no-results">لا توجد ملفات تطابق البحث</td></tr>`
  return filtered.map((row) => `<tr class="${row.status === 'review' ? 'row-warning' : ''}" data-file="${escapeHtml(row.name)}"><td class="name-cell"><button class="row-toggle" type="button" data-expand="${escapeHtml(row.name)}" aria-expanded="${state.expandedFile === row.name}">${icon('video', 17)}<span>${escapeHtml(row.name)}</span>${icon('chevron', 16)}</button></td><td class="latin">${row.before.toFixed(1)} GB</td><td class="latin">${row.after.toFixed(1)} GB</td><td class="latin">${savingsPercent(row.before, row.after).toFixed(0)}%</td><td>${status(row)}</td></tr>${state.expandedFile === row.name ? `<tr class="detail-row"><td colspan="5"><div class="detail-note">${icon(row.status === 'review' ? 'alert' : 'help', 18)}<div><strong>${row.status === 'review' ? 'لم يُعتمد ملف جديد لأن حجمه لم ينخفض' : row.status === 'unchanged' ? 'الملف الأصلي مناسب ولم يحتج إلى إعادة ضغط' : 'اكتملت المعالجة والتحقق'}</strong><span>الترميز المستخدم: <b dir="ltr">${escapeHtml(row.codec)}</b> <span class="detail-sep">|</span> الحجم الأصلي محفوظ دون تغيير</span></div></div></td></tr>` : ''}`).join('')
}

function renderResults(): string {
  if (isDesktop) return renderNativeResults()
  return `<div class="page page-results">${screenHeading('اكتملت معالجة الدفعة', 'الملفات الأصلية محفوظة دون تغيير')}
    <div class="results-overview"><div class="result-highlight"><strong class="latin">318.6 GB</strong><span>مساحة تم توفيرها بعد التحقق</span></div><dl class="results-counts"><div><dd class="latin">1,200</dd><dt>مكتمل</dt></div><div><dd class="latin">40</dd><dt>دون تغيير</dt></div><div><dd class="latin">8</dd><dt>تحتاج مراجعة</dt></div></dl><div class="result-actions">${button(`${icon('folder', 18)} فتح مجلد النتائج`, 'open-results-folder', 'primary')}<div class="menu-wrap">${button(`تصدير التقرير ${icon('chevron', 16)}`, 'toggle-report-menu', 'outline', `aria-expanded="${state.reportMenu}"`)}${state.reportMenu ? `<div class="export-menu" role="menu"><button role="menuitem" data-export="csv">CSV</button><button role="menuitem" data-export="excel">Excel (CSV)</button><button role="menuitem" data-export="html">HTML</button><button role="menuitem" data-export="pdf">PDF (طباعة)</button></div>` : ''}</div></div></div>
    <div class="section-title results-title"><h2>نتائج الملفات</h2><div class="search-box">${icon('search', 19)}<input id="file-search" type="search" placeholder="البحث في الملفات" value="${escapeHtml(state.search)}" aria-label="البحث في الملفات" /><button type="button" data-action="clear-search" aria-label="مسح البحث" ${state.search ? '' : 'hidden'}>${icon('close', 15)}</button></div></div>
    <div class="table-scroll"><table class="data-table results-table"><thead><tr><th>الملف</th><th>قبل</th><th>بعد</th><th>التوفير</th><th>الحالة</th></tr></thead><tbody id="result-table-body">${resultRows()}</tbody></table></div>
    <p class="data-disclaimer">القيم المعروضة نموذجية لتجربة الواجهة</p>
  </div>`
}

function renderSettings(): string {
  const options: { id: Resource; name: string; description: string }[] = [
    { id: 'low', name: 'منخفض', description: 'للاستخدام أثناء العمل على مهام أخرى' },
    { id: 'balanced', name: 'متوازن', description: 'استهلاك مناسب لمعظم الأجهزة' },
    { id: 'maximum', name: 'أقصى سرعة', description: 'يخصص موارد أكثر لعملية الضغط' },
  ]
  return `<div class="page page-settings">${screenHeading('الإعدادات', 'اضبط طريقة استخدام موارد جهازك')}
    <section class="settings-section"><h2>استهلاك موارد الجهاز</h2><div class="resource-options">${options.map((option) => `<label class="resource-option ${state.resource === option.id ? 'selected' : ''}"><input type="radio" name="resource" value="${option.id}" ${state.resource === option.id ? 'checked' : ''} /><span><strong>${option.name}</strong><small>${option.description}</small></span></label>`).join('')}</div></section>
    <section class="settings-section"><h2>تسريع الترميز</h2>${toggle()}<p class="setting-description">يستفيد من إمكانيات الجهاز عند توفرها، وقد تختلف السرعة وحجم الملف حسب الترميز</p></section>
    <footer class="page-actions">${button('حفظ الإعدادات', 'save-settings', 'primary')}</footer>
  </div>`
}

function dialogMarkup(): string {
  if (state.dialog === 'output') return `<dialog id="app-dialog" class="app-dialog" aria-labelledby="dialog-heading"><form method="dialog"><h2 id="dialog-heading">مجلد حفظ النتائج</h2><p>اكتب اسم المجلد الذي تود ظهوره في المعاينة</p><label for="output-directory-name">اسم المجلد</label><input id="output-directory-name" value="${escapeHtml(state.outputName)}" required maxlength="120" /><div class="dialog-actions">${button('اعتماد', 'confirm-output', 'primary')}${button('إلغاء', 'close-dialog', 'outline')}</div></form></dialog>`
  if (state.dialog === 'cancel') return `<dialog id="app-dialog" class="app-dialog" aria-labelledby="dialog-heading"><h2 id="dialog-heading">${isDesktop ? 'إلغاء المهام المنتظرة؟' : 'إلغاء معاينة المعالجة؟'}</h2><p>${isDesktop ? 'المهام الحالية ستنتهي أولًا، ولن تبدأ مهام جديدة، والملفات الأصلية ستظل محفوظة' : 'ستعود إلى إعداد الدفعة، ولن تتغير أي ملفات أصلية'}</p><div class="dialog-actions">${button(isDesktop ? 'تأكيد الإلغاء' : 'العودة للدفعة', 'confirm-cancel', 'danger')}${button('متابعة المعالجة', 'close-dialog', 'outline')}</div></dialog>`
  if (state.dialog === 'results') return `<dialog id="app-dialog" class="app-dialog" aria-labelledby="dialog-heading"><h2 id="dialog-heading">مجلد النتائج</h2><p>هذه نسخة لمعاينة الواجهة، ولم تُنتج ملفات مضغوطة أو يُنشأ مجلد نتائج على جهازك بعد</p><div class="dialog-actions">${button('فهمت', 'close-dialog', 'primary')}</div></dialog>`
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

function render(): void {
  root.innerHTML = `<div class="app-shell">${sidebar()}<div class="workspace"><div class="window-bar"><span>مرئي <span class="bar-separator">/</span> إدارة ملفات الفيديو</span><span class="window-status"><span class="availability-dot"></span> ${isDesktop ? 'تشغيل محلي' : 'معاينة تفاعلية'}</span></div><main id="main-content" tabindex="-1">${state.notice ? `<div class="native-notice" role="status">${escapeHtml(state.notice)}</div>` : ''}${currentScreen()}</main></div></div>${dialogMarkup()}`
  if (state.dialog) {
    const dialog = document.querySelector<HTMLDialogElement>('#app-dialog')
    dialog?.showModal()
    dialog?.addEventListener('close', () => { state.dialog = null }, { once: true })
  }
}

function changeScreen(screen: Screen): void {
  if (state.screen === 'progress' && screen !== 'progress') stopTimer()
  state.screen = screen
  state.reportMenu = false
  state.dialog = null
  render()
  document.querySelector<HTMLElement>('#main-content')?.focus({ preventScroll: true })
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
    if (state.progress === 100) { stopTimer(); state.paused = true; changeScreen('results') }
    else render()
  }, 1400)
}

function download(name: string, content: string, type: string): void {
  const url = URL.createObjectURL(new Blob([content], { type }))
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = name
  anchor.click()
  window.setTimeout(() => URL.revokeObjectURL(url), 1000)
}

async function exportReport(format: string): Promise<void> {
  state.reportMenu = false
  if (isDesktop) {
    try {
      if (format === 'pdf') window.print()
      else {
        const type = format === 'excel' ? 'csv' : format
        if (type !== 'csv' && type !== 'html') throw new Error('صيغة غير مدعومة')
        const content = await nativeReportContent(type)
        download(`mar2i-report.${type}`, content, type === 'csv' ? 'text/csv;charset=utf-8' : 'text/html;charset=utf-8')
      }
    } catch (error) { showError(error) }
    render()
    return
  }
  if (format === 'csv' || format === 'excel') {
    download('mar2i-report.csv', reportCsv(sampleFiles), 'text/csv;charset=utf-8')
  } else if (format === 'html') {
    const rows = sampleFiles.map((row) => `<tr><td>${escapeHtml(row.name)}</td><td>${row.before}</td><td>${row.after}</td><td>${savingsPercent(row.before, row.after)}%</td></tr>`).join('')
    download('mar2i-report.html', `<!doctype html><html lang="ar" dir="rtl"><meta charset="utf-8"><title>تقرير مرئي</title><style>body{font-family:Tahoma,sans-serif;margin:48px;color:#243330}table{border-collapse:collapse;width:100%}td,th{border-bottom:1px solid #ddd;padding:12px;text-align:right}</style><h1>نتائج الملفات</h1><table><thead><tr><th>الملف</th><th>قبل GB</th><th>بعد GB</th><th>التوفير</th></tr></thead><tbody>${rows}</tbody></table></html>`, 'text/html;charset=utf-8')
  } else if (format === 'pdf') window.print()
  render()
}

async function selectOutputFolder(): Promise<void> {
  const picker = (window as Window & { showDirectoryPicker?: () => Promise<{ name: string }> }).showDirectoryPicker
  if (picker) {
    try {
      const handle = await picker.call(window)
      state.outputName = handle.name
      render()
      return
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return
    }
  }
  state.dialog = 'output'
  render()
}

async function pollNative() {
  try {
    const result = await nativeStatus();
    state.batch = result;
    if (result.error) state.notice = result.error;
    if (!result.running && result.files > 0 && state.screen === 'progress') {
      if (nativeTimer !== null) clearInterval(nativeTimer);
      nativeTimer = null;
      changeScreen('results');
    } else if (state.screen === 'progress' || state.screen === 'results') {
      render();
    }
  } catch (error) {
    if (nativeTimer !== null) clearInterval(nativeTimer);
    nativeTimer = null;
    showError(error);
  }
}

async function handleNativeAction(action: string): Promise<boolean> {
  if (action === 'select-source' || action === 'select-output') {
    try {
      const path = await pickDirectory();
      if (!path) return true;
      if (action === 'select-source') { state.sourcePath = path; state.sourceName = basename(path); }
      else { state.outputPath = path; state.outputName = basename(path); }
      state.analysis = null;
      state.notice = null;
      render();
    } catch (error) { showError(error); }
    return true;
  }
  if (action === 'analyze') {
    if (!state.sourcePath || !state.outputPath) {
      showError('اختر مجلد المصدر ومجلد الحفظ أولًا');
      return true;
    }
    state.notice = 'جارٍ تحليل الملفات باستخدام FFprobe';
    render();
    try {
      const scan = await analyzeDirectory(state.sourcePath,state.outputPath,state.mode);
      state.analysis = scan;
      state.sourceCount = scan.total;
      state.notice = null;
      changeScreen('review');
    } catch (error) { showError(error); }
    return true;
  }
  if (action === 'start-preview') {
    if (!state.sourcePath || !state.outputPath || !state.analysis) { showError('يرجى تحليل الملفات أولًا'); return true; }
    try {
      await startNativeBatch(state.sourcePath,state.outputPath,state.mode,state.resource,state.codec);
      state.batch = null;
      state.notice = null;
      changeScreen('progress');
      if (nativeTimer !== null) clearInterval(nativeTimer);
      nativeTimer = setInterval(() => { void pollNative(); }, 900);
      void pollNative();
    } catch (error) { showError(error); }
    return true;
  }
  if (action === 'pause-preview') {
    try { await nativeControl(state.batch?.paused ? 'resume' : 'pause'); await pollNative(); }
    catch (error) { showError(error); }
    return true;
  }
  if (action === 'confirm-cancel') {
    state.dialog = null;
    try { await nativeControl('cancel'); await pollNative(); }
    catch (error) { showError(error); }
    return true;
  }
  if (action === 'open-results-folder') {
    try { await openNativeResults(); }
    catch (error) { showError(error); }
    return true;
  }
  return false;
}

root.addEventListener('click', async (event) => {
  const target = event.target as HTMLElement
  const nav = target.closest<HTMLElement>('[data-screen]')
  if (nav) { changeScreen(nav.dataset.screen as Screen); return }
  const mode = target.closest<HTMLElement>('[data-mode]')
  if (mode) { state.mode = mode.dataset.mode as Mode; state.analysis = null; state.notice = null; render(); return }
  const help = target.closest<HTMLElement>('[data-help]')
  if (help) { state.help = help.dataset.help as CodecId; render(); return }
  const expand = target.closest<HTMLElement>('[data-expand]')
  if (expand) { state.expandedFile = state.expandedFile === expand.dataset.expand ? null : expand.dataset.expand!; render(); return }
  const exportButton = target.closest<HTMLElement>('[data-export]')
  if (exportButton) { void exportReport(exportButton.dataset.export!); return }
  const action = target.closest<HTMLElement>('[data-action]')?.dataset.action
  if (!action) return
  if (isDesktop && await handleNativeAction(action)) return
  switch (action) {
    case 'select-source': document.querySelector<HTMLInputElement>('#source-picker')?.click(); break
    case 'select-output': await selectOutputFolder(); break
    case 'analyze': changeScreen('review'); break
    case 'go-options': changeScreen('options'); break
    case 'back-new': case 'reset-new': changeScreen('new'); break
    case 'show-selected-help': state.help = state.codec; changeScreen('options'); break
    case 'save-options': changeScreen('new'); break
    case 'save-settings': changeScreen('new'); break
    case 'start-preview': changeScreen('progress'); startTimer(); break
    case 'pause-preview': state.paused ? startTimer() : (state.paused = true, stopTimer(), render()); break
    case 'cancel-preview': state.dialog = 'cancel'; render(); break
    case 'confirm-cancel': stopTimer(); state.paused = true; state.progress = 34; changeScreen('new'); break
    case 'confirm-output': {
      const value = document.querySelector<HTMLInputElement>('#output-directory-name')?.value.trim()
      if (value) { state.outputName = value; state.dialog = null; render() }
      break
    }
    case 'close-dialog': state.dialog = null; render(); break
    case 'toggle-report-menu': state.reportMenu = !state.reportMenu; render(); break
    case 'clear-search': state.search = ''; render(); document.querySelector<HTMLInputElement>('#file-search')?.focus(); break
    case 'open-results-folder': state.dialog = 'results'; render(); break
  }
})

root.addEventListener('change', (event) => {
  const target = event.target as HTMLInputElement
  if (target.id === 'source-picker' && target.files?.length) {
    const first = target.files[0]
    state.sourceName = first.webkitRelativePath.split('/')[0] || first.name
    state.sourceCount = [...target.files].filter((file) => /\.(mp4|mov|mkv|avi|webm|mxf|mts|m2ts)$/i.test(file.name)).length
    render()
  } else if (target.name === 'codec') { state.codec = target.value as CodecId; state.help = state.codec; render() }
  else if (target.id === 'hardware') { state.hardware = target.checked; render() }
  else if (target.name === 'resource') { state.resource = target.value as Resource; render() }
})

root.addEventListener('input', (event) => {
  const input = event.target as HTMLInputElement
  if (input.id === 'file-search') {
    state.search = input.value
    document.querySelector('#result-table-body')!.innerHTML = resultRows()
    const clear = document.querySelector<HTMLButtonElement>('[data-action="clear-search"]')
    if (clear) clear.hidden = !input.value
  }
})

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && state.reportMenu) { state.reportMenu = false; render() }
})

render()
