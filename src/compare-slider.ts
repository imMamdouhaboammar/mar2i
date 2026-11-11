/**
 * Visual A/B Frame Comparator Component for Mar2i (مرئي)
 * Inspired by compressO's MediaOutputCompareSlider architecture.
 * 
 * Provides an accessible, responsive, zero-dependency split slider
 * allowing Ministry inspectors to review visual fidelity side-by-side
 * with real-time SSIM index overlay, Loupe Magnifier, and Heatmap mode.
 * 
 * Fully supports RTL (Arabic) layout direction and keyboard navigation.
 */

export interface SceneFrame {
  id: string
  title: string
  category: 'high-motion' | 'low-light' | 'typography'
  beforeImage: string
  afterImage: string
  ssimScore: number
}

export type CompareSliderOptions = {
  container: HTMLElement
  beforeImage: string
  afterImage: string
  beforeLabel?: string
  afterLabel?: string
  ssimScore?: number
  initialPosition?: number // 0 to 100
  scenes?: SceneFrame[]
  onSceneChange?: (sceneId: string) => void
}

export class CompareSlider {
  private container: HTMLElement
  private sliderHandle: HTMLElement | null = null
  private afterLayer: HTMLElement | null = null
  private position: number
  private isDragging = false
  private isLoupeActive = false
  private isHeatmapActive = false
  private currentSceneIndex = 0
  private scenes: SceneFrame[] = []
  private options: CompareSliderOptions

  constructor(options: CompareSliderOptions) {
    this.options = options
    this.container = options.container
    this.position = options.initialPosition ?? 50
    this.scenes = options.scenes ?? [
      {
        id: 'scene-hajj-motion',
        title: 'مشهد حركة عالي: طواف الكعبة المشرفة',
        category: 'high-motion',
        beforeImage: options.beforeImage,
        afterImage: options.afterImage,
        ssimScore: options.ssimScore ?? 0.992,
      },
      {
        id: 'scene-night-light',
        title: 'مشهد إضاءة ليلية وتباين: ساحات المسجد الحرام',
        category: 'low-light',
        beforeImage: options.beforeImage,
        afterImage: options.afterImage,
        ssimScore: 0.987,
      },
      {
        id: 'scene-typography',
        title: 'مشهد نصوص ولافتات إرشادية بدقة عالية',
        category: 'typography',
        beforeImage: options.beforeImage,
        afterImage: options.afterImage,
        ssimScore: 0.996,
      },
    ]
    this.render()
    this.attachEvents()
  }

  public setPosition(pos: number): void {
    this.position = Math.max(0, Math.min(100, pos))
    this.updateLayers()
  }

  public setScene(index: number): void {
    if (index >= 0 && index < this.scenes.length) {
      this.currentSceneIndex = index
      this.options.onSceneChange?.(this.scenes[index].id)
      this.render()
      this.attachEvents()
    }
  }

  private render(): void {
    const activeScene = this.scenes[this.currentSceneIndex]
    const beforeLabel = this.options.beforeLabel ?? 'المصدر الأصلي (ProRes 422 HQ)'
    const afterLabel = this.options.afterLabel ?? 'النسخة المضغوطة (HEVC Main 10)'
    const ssim = activeScene.ssimScore
    const ssimText = `SSIM: ${ssim.toFixed(3)} · قيمة توضيحية، وليست نتيجة فحص حقيقي`

    this.container.innerHTML = `
      <div class="compare-component-root" dir="rtl">
        <!-- Scene Switcher & View Modes Bar -->
        <div class="compare-toolbar">
          <div class="scene-selector-group">
            <span class="toolbar-label">إطار المقارنة:</span>
            <div class="scene-pills" role="radiogroup" aria-label="اختيار مشهد المقارنة">
              ${this.scenes.map((s, idx) => `
                <button class="scene-pill ${idx === this.currentSceneIndex ? 'is-active' : ''}" type="button" role="radio" aria-checked="${idx === this.currentSceneIndex}" data-scene-idx="${idx}">
                  ${s.title.split(':')[0]}
                </button>
              `).join('')}
            </div>
          </div>

          <div class="compare-modes-group">
            <button class="mode-toggle-btn ${this.isLoupeActive ? 'is-active' : ''}" type="button" id="btn-toggle-loupe" title="تفعيل عدسة التكبير الفاحصة (2x)">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8"/>
                <line x1="21" y1="21" x2="16.65" y2="16.65"/>
                <line x1="11" y1="8" x2="11" y2="14"/>
                <line x1="8" y1="11" x2="14" y2="11"/>
              </svg>
              <span>عدسة التكبير (2x)</span>
            </button>

            <button class="mode-toggle-btn ${this.isHeatmapActive ? 'is-active' : ''}" type="button" id="btn-toggle-heatmap" title="تفعيل خريطة الفروق البصرية">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z"/>
                <circle cx="12" cy="12" r="3"/>
              </svg>
              <span>خريطة التباين</span>
            </button>
          </div>
        </div>

        <!-- Main Split Canvas Area -->
        <div class="compare-wrapper ${this.isHeatmapActive ? 'is-heatmap' : ''}" role="region" aria-label="أداة مقارنة الجودة البصرية">
          <!-- Before Layer (Base) -->
          <div class="compare-layer compare-layer-before">
            <img src="${activeScene.beforeImage}" alt="${beforeLabel}" class="compare-image" />
            <span class="compare-badge compare-badge-before">${beforeLabel}</span>
          </div>

          <!-- After Layer (Clipped Overlay) -->
          <div class="compare-layer compare-layer-after" style="clip-path: inset(0 0 0 ${this.position}%);">
            <img src="${activeScene.afterImage}" alt="${afterLabel}" class="compare-image" />
            <span class="compare-badge compare-badge-after">${afterLabel}</span>
          </div>

          <!-- Loupe Magnifier Lens -->
          <div class="compare-loupe-lens ${this.isLoupeActive ? 'is-visible' : ''}" id="compare-loupe">
            <div class="loupe-preview-before">
              <img src="${activeScene.beforeImage}" alt="تكبير الأصل" />
            </div>
            <div class="loupe-preview-after">
              <img src="${activeScene.afterImage}" alt="تكبير المضغوط" />
            </div>
            <span class="loupe-crosshair"></span>
          </div>

          <!-- SSIM Quality Score Pill -->
          <div class="compare-ssim-pill">
            <span class="ssim-dot"></span>
            <span class="ssim-label">${ssimText}</span>
          </div>

          <!-- Split Divider Handle -->
          <div 
            class="compare-handle" 
            role="slider" 
            tabindex="0"
            aria-label="مقبض المقارنة البصرية"
            aria-valuemin="0" 
            aria-valuemax="100" 
            aria-valuenow="${this.position}"
            style="left: ${this.position}%;"
          >
            <div class="compare-handle-line"></div>
            <div class="compare-handle-button" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                <path d="M8.5 7l-5 5 5 5V7zm7 10l5-5-5-5v10z"/>
              </svg>
            </div>
          </div>
        </div>

        <div class="compare-instruction-caption">
          <span>اسحب المقبض يمينًا ويسارًا للمقارنة التفاعلية، أو استخدم مفاتيح الأسهم (← / →) للتنقل بدقة 1%</span>
        </div>
      </div>
    `

    this.sliderHandle = this.container.querySelector('.compare-handle')
    this.afterLayer = this.container.querySelector('.compare-layer-after')
  }

  private updateLayers(): void {
    if (!this.sliderHandle || !this.afterLayer) return
    this.sliderHandle.style.left = `${this.position}%`
    this.sliderHandle.setAttribute('aria-valuenow', this.position.toString())
    this.afterLayer.style.clipPath = `inset(0 0 0 ${this.position}%)`
  }

  private attachEvents(): void {
    if (!this.sliderHandle || !this.afterLayer) return

    const wrapper = this.container.querySelector('.compare-wrapper') as HTMLElement
    const loupe = this.container.querySelector('#compare-loupe') as HTMLElement
    if (!wrapper) return

    // Scene pill click events
    const sceneBtns = this.container.querySelectorAll<HTMLButtonElement>('.scene-pill')
    sceneBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        const idx = parseInt(btn.getAttribute('data-scene-idx') ?? '0', 10)
        this.setScene(idx)
      })
    })

    // Loupe toggle
    const loupeBtn = this.container.querySelector<HTMLButtonElement>('#btn-toggle-loupe')
    loupeBtn?.addEventListener('click', () => {
      this.isLoupeActive = !this.isLoupeActive
      this.render()
      this.attachEvents()
    })

    // Heatmap toggle
    const heatmapBtn = this.container.querySelector<HTMLButtonElement>('#btn-toggle-heatmap')
    heatmapBtn?.addEventListener('click', () => {
      this.isHeatmapActive = !this.isHeatmapActive
      this.render()
      this.attachEvents()
    })

    // Loupe pointer movement
    if (loupe) {
      wrapper.addEventListener('pointermove', (e: PointerEvent) => {
        if (!this.isLoupeActive) return
        const rect = wrapper.getBoundingClientRect()
        const x = e.clientX - rect.left
        const y = e.clientY - rect.top
        loupe.style.left = `${x}px`
        loupe.style.top = `${y}px`

        const beforeImg = loupe.querySelector('.loupe-preview-before img') as HTMLElement
        const afterImg = loupe.querySelector('.loupe-preview-after img') as HTMLElement
        if (beforeImg && afterImg) {
          const factor = 2.5
          const imgX = -(x * factor - 60)
          const imgY = -(y * factor - 60)
          beforeImg.style.transform = `translate(${imgX}px, ${imgY}px) scale(${factor})`
          afterImg.style.transform = `translate(${imgX}px, ${imgY}px) scale(${factor})`
        }
      })
    }

    const onPointerDown = (e: PointerEvent) => {
      if ((e.target as HTMLElement).closest('#compare-loupe') && this.isLoupeActive) return
      this.isDragging = true
      wrapper.setPointerCapture(e.pointerId)
      this.updateFromPointer(e, wrapper)
    }

    const onPointerMove = (e: PointerEvent) => {
      if (!this.isDragging) return
      this.updateFromPointer(e, wrapper)
    }

    const onPointerUp = (e: PointerEvent) => {
      if (!this.isDragging) return
      this.isDragging = false
      try {
        wrapper.releasePointerCapture(e.pointerId)
      } catch {}
    }

    wrapper.addEventListener('pointerdown', onPointerDown)
    wrapper.addEventListener('pointermove', onPointerMove)
    wrapper.addEventListener('pointerup', onPointerUp)
    wrapper.addEventListener('pointercancel', onPointerUp)

    // Keyboard accessibility for handle
    this.sliderHandle.addEventListener('keydown', (e: KeyboardEvent) => {
      let step = 1
      if (e.shiftKey) step = 10

      if (e.key === 'ArrowLeft') {
        e.preventDefault()
        this.setPosition(this.position - step)
      } else if (e.key === 'ArrowRight') {
        e.preventDefault()
        this.setPosition(this.position + step)
      } else if (e.key === 'Home') {
        e.preventDefault()
        this.setPosition(0)
      } else if (e.key === 'End') {
        e.preventDefault()
        this.setPosition(100)
      }
    })
  }

  private updateFromPointer(e: PointerEvent, wrapper: HTMLElement): void {
    const rect = wrapper.getBoundingClientRect()
    const x = e.clientX - rect.left
    const percent = Math.max(0, Math.min(100, (x / rect.width) * 100))
    this.setPosition(percent)
  }
}
