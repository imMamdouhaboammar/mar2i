/**
 * Aspect Ratio Studio & Visual Framing Widget for Mar2i (مرئي)
 * Inspired by compressO's MediaTransformer & Cropper architecture.
 * Adapted for Saudi Ministry of Hajj & Umrah multi-platform publishing.
 */

export interface AspectRatioPreset {
  id: string
  name: string
  ratio: number // width / height
  platform: string
  badge: string
}

export const ASPECT_PRESETS: AspectRatioPreset[] = [
  { id: '16-9', name: '16:9 شاشات العرض والتلفزيون', ratio: 16 / 9, platform: 'البث الفضائي، المؤتمرات، يوتيوب', badge: 'شائع' },
  { id: '9-16', name: '9:16 طولي لمنصات الجوال', ratio: 9 / 16, platform: 'ريلز، تيك توك، سناب شات، قصص إكس', badge: 'جوال' },
  { id: '1-1', name: '1:1 مربع متناسق', ratio: 1 / 1, platform: 'خلاصات المنشورات والإنستغرام', badge: 'مربع' },
  { id: '4-3', name: '4:3 الأرشيف التوثيقي الكلاسيكي', ratio: 4 / 3, platform: 'الوثائقيات واللقطات التاريخية', badge: 'أرشيف' },
  { id: 'original', name: 'الأبعاد الأصلية للمصدر', ratio: 0, platform: 'بدون تعديل في أبعاد الإطار', badge: 'أصل' },
]

export interface AspectStudioOptions {
  container: HTMLElement
  fileName: string
  initialPresetId?: string
  initialRotation?: number
  initialFlipped?: boolean
  onApply?: (config: { presetId: string; rotation: number; flipped: boolean; showSafeZone: boolean }) => void
  onCancel?: () => void
}

export class AspectRatioStudio {
  private container: HTMLElement
  private fileName: string
  private activePresetId: string
  private rotation: number
  private flipped: boolean
  private showSafeZone: boolean
  private options: AspectStudioOptions

  constructor(options: AspectStudioOptions) {
    this.options = options
    this.container = options.container
    this.fileName = options.fileName
    this.activePresetId = options.initialPresetId ?? '16-9'
    this.rotation = options.initialRotation ?? 0
    this.flipped = options.initialFlipped ?? false
    this.showSafeZone = true
    this.render()
  }

  public setPreset(id: string): void {
    this.activePresetId = id
    this.render()
  }

  public rotateClockwise(): void {
    this.rotation = (this.rotation + 90) % 360
    this.render()
  }

  public toggleFlip(): void {
    this.flipped = !this.flipped
    this.render()
  }

  private render(): void {
    const activePreset = ASPECT_PRESETS.find((p) => p.id === this.activePresetId) ?? ASPECT_PRESETS[0]
    
    // Calculate aspect ratio box style
    let aspectClass = 'aspect-16-9'
    if (this.activePresetId === '9-16') aspectClass = 'aspect-9-16'
    else if (this.activePresetId === '1-1') aspectClass = 'aspect-1-1'
    else if (this.activePresetId === '4-3') aspectClass = 'aspect-4-3'
    else if (this.activePresetId === 'original') aspectClass = 'aspect-16-9'

    this.container.innerHTML = `
      <div class="aspect-studio-modal" role="dialog" aria-modal="true" aria-labelledby="studio-title" dir="rtl">
        <div class="aspect-studio-backdrop"></div>
        <div class="aspect-studio-card">
          <header class="aspect-studio-header">
            <div>
              <h2 id="studio-title">استوديو ملاءمة الأبعاد والتأطير البصري</h2>
              <p class="aspect-studio-sub">ضبط أبعاد الفيديو (${this.fileName}) للنشر الحكومي والإعلامي</p>
            </div>
            <button class="aspect-studio-close-btn" type="button" aria-label="إغلاق الاستوديو">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </header>

          <main class="aspect-studio-body">
            <!-- Left / Center: Interactive Visual Frame Canvas -->
            <div class="aspect-canvas-viewport">
              <div class="aspect-canvas-wrapper ${aspectClass}">
                <div class="aspect-canvas-preview" style="transform: rotate(${this.rotation}deg) scaleX(${this.flipped ? -1 : 1});">
                  <!-- Simulated High-Quality Holy Mosque Video Frame -->
                  <div class="aspect-mock-content">
                    <div class="aspect-mock-bg"></div>
                    <div class="aspect-mock-overlay">
                      <span class="aspect-badge-floating">${activePreset.name}</span>
                    </div>
                  </div>
                </div>

                <!-- TV / Social Title Safe Margins Overlay -->
                ${this.showSafeZone ? `
                  <div class="aspect-safe-zone action-safe" title="منطقة الأمان للإجراءات (Action Safe 93%)">
                    <div class="aspect-safe-zone title-safe" title="منطقة الأمان للشعارات والنصوص (Title Safe 90%)">
                      <span class="safe-zone-tag">منطقة أمان هوية الوزارة</span>
                    </div>
                  </div>
                ` : ''}
              </div>

              <!-- Quick Canvas Rotation & Flip Toolbar -->
              <div class="aspect-tools-toolbar">
                <button class="tool-btn" type="button" id="btn-rotate" title="تدوير 90 درجة مع عقارب الساعة">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/>
                  </svg>
                  <span>تدوير 90° (${this.rotation}°)</span>
                </button>
                <button class="tool-btn ${this.flipped ? 'is-active' : ''}" type="button" id="btn-flip" title="عكس أفقي (مرآة)">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="16 3 21 3 21 8"/>
                    <line x1="4" y1="20" x2="21" y2="3"/>
                    <polyline points="21 16 21 21 16 21"/>
                    <line x1="15" y1="15" x2="21" y2="21"/>
                    <line x1="4" y1="4" x2="9" y2="9"/>
                  </svg>
                  <span>عكس أفقي (مرآة)</span>
                </button>
                <button class="tool-btn ${this.showSafeZone ? 'is-active' : ''}" type="button" id="btn-safe-zone" title="إظهار/إخفاء خطوط الأمان للنصوص">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                    <rect x="7" y="7" width="10" height="10"/>
                  </svg>
                  <span>خطوط الأمان</span>
                </button>
              </div>
            </div>

            <!-- Right: Preset Selection Sidebar -->
            <div class="aspect-presets-sidebar">
              <h3>قوالب الأبعاد المعتمدة:</h3>
              <div class="presets-cards-stack">
                ${ASPECT_PRESETS.map((p) => `
                  <button class="preset-card ${p.id === this.activePresetId ? 'is-selected' : ''}" type="button" data-preset-id="${p.id}">
                    <div class="preset-card-head">
                      <strong>${p.name}</strong>
                      <span class="preset-badge">${p.badge}</span>
                    </div>
                    <p class="preset-card-desc">${p.platform}</p>
                  </button>
                `).join('')}
              </div>
            </div>
          </main>

          <footer class="aspect-studio-footer">
            <button class="studio-btn primary" type="button" id="btn-apply-aspect">
              تطبيق الأبعاد والتأطير (${activePreset.name.split(' ')[0]})
            </button>
            <button class="studio-btn outline" type="button" id="btn-cancel-aspect">
              إلغاء
            </button>
          </footer>
        </div>
      </div>
    `

    this.attachEvents()
  }

  private attachEvents(): void {
    const closeBtn = this.container.querySelector<HTMLButtonElement>('.aspect-studio-close-btn')
    const cancelBtn = this.container.querySelector<HTMLButtonElement>('#btn-cancel-aspect')
    const backdrop = this.container.querySelector<HTMLElement>('.aspect-studio-backdrop')
    const applyBtn = this.container.querySelector<HTMLButtonElement>('#btn-apply-aspect')

    const closeHandler = () => {
      this.options.onCancel?.()
      this.container.innerHTML = ''
    }

    closeBtn?.addEventListener('click', closeHandler)
    cancelBtn?.addEventListener('click', closeHandler)
    backdrop?.addEventListener('click', closeHandler)

    applyBtn?.addEventListener('click', () => {
      this.options.onApply?.({
        presetId: this.activePresetId,
        rotation: this.rotation,
        flipped: this.flipped,
        showSafeZone: this.showSafeZone,
      })
      this.container.innerHTML = ''
    })

    const rotateBtn = this.container.querySelector<HTMLButtonElement>('#btn-rotate')
    rotateBtn?.addEventListener('click', () => {
      this.rotateClockwise()
    })

    const flipBtn = this.container.querySelector<HTMLButtonElement>('#btn-flip')
    flipBtn?.addEventListener('click', () => {
      this.toggleFlip()
    })

    const safeBtn = this.container.querySelector<HTMLButtonElement>('#btn-safe-zone')
    safeBtn?.addEventListener('click', () => {
      this.showSafeZone = !this.showSafeZone
      this.render()
    })

    const presetCards = this.container.querySelectorAll<HTMLButtonElement>('.preset-card')
    presetCards.forEach((card) => {
      card.addEventListener('click', () => {
        const id = card.getAttribute('data-preset-id')
        if (id) this.setPreset(id)
      })
    })
  }
}
