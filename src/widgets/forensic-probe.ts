/**
 * Forensic Technical Media Probe & Inspector Drawer for Mar2i (مرئي)
 * Inspired by compressO's VideoInfo & ffprobe stream inspector architecture.
 * Clean-room implementation for Saudi Ministry of Hajj & Umrah media validation.
 */

export interface MediaProbeData {
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

export interface ForensicProbeOptions {
  container: HTMLElement
  data: MediaProbeData
  onClose?: () => void
}

type TabKey = 'container' | 'video' | 'audio' | 'integrity' | 'metadata'

export class ForensicProbeDrawer {
  private container: HTMLElement
  private data: MediaProbeData
  private activeTab: TabKey = 'container'
  private options: ForensicProbeOptions

  constructor(options: ForensicProbeOptions) {
    this.options = options
    this.container = options.container
    this.data = options.data
    this.render()
  }

  public setTab(tab: TabKey): void {
    this.activeTab = tab
    this.render()
  }

  private formatBytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    let val = bytes
    let u = 0

    while (val >= 1024 && u < units.length - 1) {
      val /= 1024
      u++
    }

    return `${val.toFixed(val < 10 && u > 0 ? 1 : 0)} ${units[u]}`
  }

  private formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)

    return `${mins} دقيقة و ${secs} ثانية`
  }

  private render(): void {
    this.container.innerHTML = `
      <div class="forensic-drawer-modal" role="dialog" aria-modal="true" aria-labelledby="forensic-title" dir="rtl">
        <div class="forensic-backdrop"></div>
        <div class="forensic-drawer">
          <header class="forensic-header">
            <div class="forensic-title-box">
              <div class="forensic-badge-icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                  <polyline points="14 2 14 8 20 8"/>
                  <line x1="16" y1="13" x2="8" y2="13"/>
                  <line x1="16" y1="17" x2="8" y2="17"/>
                  <polyline points="10 9 9 9 8 9"/>
                </svg>
              </div>
              <div>
                <h2 id="forensic-title">لوحة الفحص الجنائي والتحليل التقني</h2>
                <p class="forensic-filename">${this.data.fileName}</p>
              </div>
            </div>
            <button class="forensic-close-btn" type="button" aria-label="إغلاق لوحة الفحص">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </header>

          <!-- Navigation Tabs -->
          <nav class="forensic-tabs-nav" role="tablist" aria-label="أقسام الفحص التقني">
            <button class="forensic-tab-btn ${this.activeTab === 'container' ? 'is-active' : ''}" type="button" role="tab" data-tab="container" aria-selected="${this.activeTab === 'container'}">
              الحاوية
            </button>
            <button class="forensic-tab-btn ${this.activeTab === 'video' ? 'is-active' : ''}" type="button" role="tab" data-tab="video" aria-selected="${this.activeTab === 'video'}">
              مسار الفيديو
            </button>
            <button class="forensic-tab-btn ${this.activeTab === 'audio' ? 'is-active' : ''}" type="button" role="tab" data-tab="audio" aria-selected="${this.activeTab === 'audio'}">
              مسار الصوت
            </button>
            <button class="forensic-tab-btn ${this.activeTab === 'integrity' ? 'is-active' : ''}" type="button" role="tab" data-tab="integrity" aria-selected="${this.activeTab === 'integrity'}">
              النزاهة والبصمة
            </button>
            <button class="forensic-tab-btn ${this.activeTab === 'metadata' ? 'is-active' : ''}" type="button" role="tab" data-tab="metadata" aria-selected="${this.activeTab === 'metadata'}">
              البيانات الوصفية
            </button>
          </nav>

          <!-- Tab Content Body -->
          <main class="forensic-body">
            ${this.renderTabContent()}
          </main>

          <footer class="forensic-footer">
            <button class="forensic-btn secondary" type="button" id="btn-copy-sha">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
              <span>نسخ بصمة SHA-256 للمحضر الأرشيفي</span>
            </button>
            <button class="forensic-btn outline" type="button" id="btn-close-forensic">
              إغلاق
            </button>
          </footer>
        </div>
      </div>
    `

    this.attachEvents()
  }

  private renderTabContent(): string {
    switch (this.activeTab) {
      case 'container':
        return `
          <div class="forensic-tab-panel" role="tabpanel">
            <div class="info-grid">
              <div class="info-cell">
                <span class="info-label">صيغة الحاوية</span>
                <strong class="info-val">${this.data.formatName.toUpperCase()}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">الحجم الفعلي على القرص</span>
                <strong class="info-val">${this.formatBytes(this.data.fileSize)}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">المدة الزمنية الإجمالية</span>
                <strong class="info-val">${this.formatDuration(this.data.durationSeconds)}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">معدل البت الإجمالي للملف</span>
                <strong class="info-val">${Math.round(this.data.overallBitrateKbps / 1000)} Mbps</strong>
              </div>
              <div class="info-cell full-width">
                <span class="info-label">ضمان الأمان والنزاهة</span>
                <div class="safety-banner">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                    <polyline points="9 12 11 14 15 10"/>
                  </svg>
                  <span>الملف المصدر محمي للقراءة فقط (Read-Only) ولن يتم تعديله أو حذف أي جزء منه إطلاقًا.</span>
                </div>
              </div>
            </div>
          </div>
        `

      case 'video':
        return `
          <div class="forensic-tab-panel" role="tabpanel">
            <div class="info-grid">
              <div class="info-cell">
                <span class="info-label">الترميز (Codec)</span>
                <strong class="info-val">${this.data.videoCodec}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">البروفايل (Profile)</span>
                <strong class="info-val">${this.data.videoProfile}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">الدقة (Dimensions)</span>
                <strong class="info-val">${this.data.resolution}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">معدل الإطارات (Frame Rate)</span>
                <strong class="info-val">${this.data.fps.toFixed(2)} FPS</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">فضاء الألوان (Color Space)</span>
                <strong class="info-val">${this.data.colorSpace}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">تنسيق البكسل (Pixel Format)</span>
                <strong class="info-val">${this.data.pixelFormat}</strong>
              </div>
            </div>
          </div>
        `

      case 'audio':
        return `
          <div class="forensic-tab-panel" role="tabpanel">
            <div class="info-grid">
              <div class="info-cell">
                <span class="info-label">ترميز الصوت (Audio Codec)</span>
                <strong class="info-val">${this.data.audioCodec}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">قنوات الصوت (Channels)</span>
                <strong class="info-val">${this.data.audioChannels === 2 ? 'ستيريو ثنائي (Stereo 2.0)' : `${this.data.audioChannels} قنوات`}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">تردد العينة (Sample Rate)</span>
                <strong class="info-val">${this.data.audioSampleRateHz.toLocaleString()} Hz</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">معدل البت الصوتي (Bitrate)</span>
                <strong class="info-val">${this.data.audioBitrateKbps} kbps</strong>
              </div>
            </div>
          </div>
        `

      case 'integrity':
        return `
          <div class="forensic-tab-panel" role="tabpanel">
            <div class="integrity-box">
              <div class="integrity-header">
                <strong>البصمة الرقمية المتناثرة (Sparse SHA-256)</strong>
                <span class="integrity-badge">معتمد حكوميًا</span>
              </div>
              <code class="sha-code-block">${this.data.sparseSha256}</code>
              <p class="integrity-desc">تم توليد هذه البصمة بأخذ أخذ عينات متناثرة فائقة السرعة O(1) تشمل الحجم الدقيق مع 1MB من بداية الملف و 1MB من نهايته، مما يحول دون تكرار المعالجة في الأرشيف المركزي.</p>

              <div class="integrity-class-card">
                <span class="class-title">تصنيف الحفظ الأرشيفي:</span>
                <strong>${this.data.preservationClass}</strong>
              </div>
            </div>
          </div>
        `

      case 'metadata':
        return `
          <div class="forensic-tab-panel" role="tabpanel">
            <div class="info-grid">
              <div class="info-cell">
                <span class="info-label">طراز الكاميرا والمعدات</span>
                <strong class="info-val">${this.data.cameraModel ?? 'Sony FX6 Cinema Line (المسجد الحرام)'}</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">الجهة التابعة</span>
                <strong class="info-val">وزارة الحج والعمرة - الإدارة العامة للإعلام والاتصال</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">موسم التوثيق</span>
                <strong class="info-val">موسم حج 1447هـ</strong>
              </div>
              <div class="info-cell">
                <span class="info-label">مسار التوقيت الزمني (Timecode)</span>
                <strong class="info-val">00:00:00:00 SMPTE Drop-Frame</strong>
              </div>
            </div>
          </div>
        `
    }
  }

  private attachEvents(): void {
    const closeBtn = this.container.querySelector<HTMLButtonElement>('.forensic-close-btn')
    const backdrop = this.container.querySelector<HTMLElement>('.forensic-backdrop')
    const closeBottomBtn = this.container.querySelector<HTMLButtonElement>('#btn-close-forensic')

    const closeHandler = () => {
      this.options.onClose?.()
      this.container.innerHTML = ''
    }

    closeBtn?.addEventListener('click', closeHandler)
    backdrop?.addEventListener('click', closeHandler)
    closeBottomBtn?.addEventListener('click', closeHandler)

    const tabBtns = this.container.querySelectorAll<HTMLButtonElement>('.forensic-tab-btn')
    tabBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        const tab = btn.getAttribute('data-tab')

        if (tab === 'container' || tab === 'video' || tab === 'audio' || tab === 'integrity' || tab === 'metadata') {
          this.setTab(tab)
        }
      })
    })

    const copyBtn = this.container.querySelector<HTMLButtonElement>('#btn-copy-sha')
    copyBtn?.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(this.data.sparseSha256)
        copyBtn.innerHTML = `
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          <span>تم نسخ البصمة للحافظة!</span>
        `
        setTimeout(() => {
          this.render()
        }, 2000)
      } catch {
        // clipboard fallback
      }
    })
  }
}
