/**
 * Precision Video Trimmer & Audio Waveform Scrubber Widget for Mar2i (مرئي)
 * Inspired by compressO's VideoTrimmerTimeline & AudioWaveform architecture.
 * Designed for Saudi Ministry of Hajj & Umrah media cutting and press extraction.
 */

export interface VideoTrimmerOptions {
  container: HTMLElement
  fileName: string
  totalDurationSeconds: number
  initialStartSeconds?: number
  initialEndSeconds?: number
  onTrimChange?: (start: number, end: number) => void
  onApply?: (start: number, end: number) => void
  onCancel?: () => void
}

export class VideoTrimmer {
  private container: HTMLElement
  private fileName: string
  private totalDuration: number
  private startTime: number
  private endTime: number
  private options: VideoTrimmerOptions
  private isDraggingStart: boolean = false
  private isDraggingEnd: boolean = false

  constructor(options: VideoTrimmerOptions) {
    this.options = options
    this.container = options.container
    this.fileName = options.fileName
    this.totalDuration = Math.max(1, options.totalDurationSeconds)
    this.startTime = options.initialStartSeconds ?? 0
    this.endTime = options.initialEndSeconds ?? this.totalDuration
    this.render()
  }

  public getSelectedRange(): { start: number; end: number; duration: number } {
    return {
      start: this.startTime,
      end: this.endTime,
      duration: Math.max(0, this.endTime - this.startTime),
    }
  }

  public setRange(start: number, end: number): void {
    this.startTime = Math.max(0, Math.min(start, this.totalDuration - 0.5))
    this.endTime = Math.min(this.totalDuration, Math.max(end, this.startTime + 0.5))
    this.render()
    this.options.onTrimChange?.(this.startTime, this.endTime)
  }

  private formatTimecode(seconds: number): string {
    const s = Math.max(0, seconds)
    const mins = Math.floor(s / 60)
    const secs = Math.floor(s % 60)
    const ms = Math.floor((s - Math.floor(s)) * 1000)
    const pad = (n: number, w = 2) => String(n).padStart(w, '0')
    return `${pad(mins)}:${pad(secs)}.${pad(ms, 3)}`
  }

  private parseTimecode(str: string): number | null {
    const parts = str.trim().split(':')
    if (parts.length === 2) {
      const mins = parseFloat(parts[0])
      const secs = parseFloat(parts[1])
      if (Number.isFinite(mins) && Number.isFinite(secs)) {
        return mins * 60 + secs
      }
    }
    return null
  }

  private render(): void {
    const trimmedDuration = Math.max(0, this.endTime - this.startTime)
    const trimPercentReduction = Math.round(((this.totalDuration - trimmedDuration) / this.totalDuration) * 100)
    const leftPercent = (this.startTime / this.totalDuration) * 100
    const rightPercent = (1 - this.endTime / this.totalDuration) * 100

    this.container.innerHTML = `
      <div class="video-trimmer-modal" role="dialog" aria-modal="true" aria-labelledby="trimmer-title" dir="rtl">
        <div class="trimmer-backdrop"></div>
        <div class="trimmer-card">
          <header class="trimmer-header">
            <div>
              <h2 id="trimmer-title">تشذيب المقطع والموجة الصوتية</h2>
              <p class="trimmer-file-label">${this.fileName} (${this.formatTimecode(this.totalDuration)})</p>
            </div>
            <button class="trimmer-close-btn" type="button" aria-label="إغلاق نافذة التشذيب">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </header>

          <main class="trimmer-body">
            <!-- Audio Waveform & Timeline Track -->
            <div class="trimmer-track-wrapper">
              <div class="trimmer-timeline-labels">
                <span>00:00.000</span>
                <span>${this.formatTimecode(this.totalDuration / 2)}</span>
                <span>${this.formatTimecode(this.totalDuration)}</span>
              </div>

              <div class="trimmer-track" id="trimmer-track-container">
                <!-- Dimmed Unselected Regions -->
                <div class="trimmer-mask-left" style="width: ${leftPercent}%;"></div>
                <div class="trimmer-mask-right" style="width: ${rightPercent}%;"></div>

                <!-- Canvas Waveform -->
                <canvas class="trimmer-waveform-canvas" width="600" height="80"></canvas>

                <!-- Active Highlight Window -->
                <div class="trimmer-active-window" style="right: ${leftPercent}%; left: ${rightPercent}%;">
                  <div class="trimmer-handle start-handle" role="slider" aria-valuenow="${this.startTime}" aria-valuemin="0" aria-valuemax="${this.totalDuration}" aria-label="نقطة البداية" tabindex="0">
                    <span class="handle-grip"></span>
                    <span class="handle-tooltip">${this.formatTimecode(this.startTime)}</span>
                  </div>
                  <div class="trimmer-handle end-handle" role="slider" aria-valuenow="${this.endTime}" aria-valuemin="0" aria-valuemax="${this.totalDuration}" aria-label="نقطة النهاية" tabindex="0">
                    <span class="handle-grip"></span>
                    <span class="handle-tooltip">${this.formatTimecode(this.endTime)}</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Timecode Numerical Inputs -->
            <div class="trimmer-inputs-row">
              <div class="trimmer-field">
                <label for="trim-start-input">نقطة البداية (In-Point)</label>
                <input id="trim-start-input" class="time-input" type="text" value="${this.formatTimecode(this.startTime)}" />
              </div>
              <div class="trimmer-duration-chip">
                <span class="chip-title">المدة المقتطعة المختارة:</span>
                <strong>${this.formatTimecode(trimmedDuration)}</strong>
                <span class="chip-savings">وفر زمني: ${trimPercentReduction}%</span>
              </div>
              <div class="trimmer-field">
                <label for="trim-end-input">نقطة النهاية (Out-Point)</label>
                <input id="trim-end-input" class="time-input" type="text" value="${this.formatTimecode(this.endTime)}" />
              </div>
            </div>

            <!-- Preset Cut Buttons -->
            <div class="trimmer-presets-row">
              <span class="presets-label">قوالب اقتطاع سريعة:</span>
              <button class="preset-pill" type="button" data-preset="full">كامل الفيديو</button>
              <button class="preset-pill" type="button" data-preset="strip-heads">حذف 5 ثوانٍ من البداية والنهاية</button>
              <button class="preset-pill" type="button" data-preset="highlight">المقطع الأوسط (60 ثانية)</button>
            </div>
          </main>

          <footer class="trimmer-footer">
            <button class="trimmer-btn primary" type="button" id="trimmer-apply-btn">
              اعتماد مجال التشذيب (${this.formatTimecode(trimmedDuration)})
            </button>
            <button class="trimmer-btn outline" type="button" id="trimmer-cancel-btn">
              إلغاء
            </button>
          </footer>
        </div>
      </div>
    `

    this.drawWaveform()
    this.attachEvents()
  }

  private drawWaveform(): void {
    const canvas = this.container.querySelector<HTMLCanvasElement>('.trimmer-waveform-canvas')
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const width = canvas.width
    const height = canvas.height
    ctx.clearRect(0, 0, width, height)

    // Draw grid lines
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)'
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.moveTo(0, height / 2)
    ctx.lineTo(width, height / 2)
    ctx.stroke()

    // Draw simulated synthetic audio waveform bars with realistic peaks
    const barsCount = 120
    const barWidth = width / barsCount
    const gradient = ctx.createLinearGradient(0, 0, 0, height)
    gradient.addColorStop(0, '#10b981')
    gradient.addColorStop(0.5, '#3b82f6')
    gradient.addColorStop(1, '#059669')

    ctx.fillStyle = gradient

    // Deterministic pseudo-random seed based on filename
    let seed = 0
    for (let i = 0; i < this.fileName.length; i++) {
      seed = (seed << 5) - seed + this.fileName.charCodeAt(i)
    }

    const pseudoRandom = (idx: number) => {
      const x = Math.sin(seed + idx * 9.876) * 10000
      return x - Math.floor(x)
    }

    for (let i = 0; i < barsCount; i++) {
      const r = pseudoRandom(i)
      const envelope = Math.sin((i / barsCount) * Math.PI) * 0.4 + 0.6
      const barHeight = Math.max(6, r * (height * 0.8) * envelope)
      const x = i * barWidth
      const y = (height - barHeight) / 2

      ctx.fillRect(x + 1, y, barWidth - 1.5, barHeight)
    }
  }

  private attachEvents(): void {
    const closeBtn = this.container.querySelector<HTMLButtonElement>('.trimmer-close-btn')
    const cancelBtn = this.container.querySelector<HTMLButtonElement>('#trimmer-cancel-btn')
    const backdrop = this.container.querySelector<HTMLElement>('.trimmer-backdrop')
    const applyBtn = this.container.querySelector<HTMLButtonElement>('#trimmer-apply-btn')

    const closeHandler = () => {
      this.options.onCancel?.()
      this.container.innerHTML = ''
    }

    closeBtn?.addEventListener('click', closeHandler)
    cancelBtn?.addEventListener('click', closeHandler)
    backdrop?.addEventListener('click', closeHandler)

    applyBtn?.addEventListener('click', () => {
      this.options.onApply?.(this.startTime, this.endTime)
      this.container.innerHTML = ''
    })

    // Preset handlers
    const presetPills = this.container.querySelectorAll<HTMLButtonElement>('.preset-pill')
    presetPills.forEach((pill) => {
      pill.addEventListener('click', () => {
        const type = pill.getAttribute('data-preset')
        if (type === 'full') {
          this.setRange(0, this.totalDuration)
        } else if (type === 'strip-heads') {
          const cut = Math.min(5, this.totalDuration / 4)
          this.setRange(cut, this.totalDuration - cut)
        } else if (type === 'highlight') {
          const mid = this.totalDuration / 2
          const half = Math.min(30, this.totalDuration / 2)
          this.setRange(Math.max(0, mid - half), Math.min(this.totalDuration, mid + half))
        }
      })
    })

    // Input handlers
    const startInput = this.container.querySelector<HTMLInputElement>('#trim-start-input')
    const endInput = this.container.querySelector<HTMLInputElement>('#trim-end-input')

    startInput?.addEventListener('change', () => {
      const parsed = this.parseTimecode(startInput.value)
      if (parsed !== null) {
        this.setRange(parsed, this.endTime)
      } else {
        startInput.value = this.formatTimecode(this.startTime)
      }
    })

    endInput?.addEventListener('change', () => {
      const parsed = this.parseTimecode(endInput.value)
      if (parsed !== null) {
        this.setRange(this.startTime, parsed)
      } else {
        endInput.value = this.formatTimecode(this.endTime)
      }
    })

    // Dragging scrubber handles
    const track = this.container.querySelector<HTMLElement>('#trimmer-track-container')
    const startHandle = this.container.querySelector<HTMLElement>('.start-handle')
    const endHandle = this.container.querySelector<HTMLElement>('.end-handle')

    if (track && startHandle && endHandle) {
      const onMouseMove = (e: MouseEvent) => {
        if (!this.isDraggingStart && !this.isDraggingEnd) return
        const rect = track.getBoundingClientRect()
        // RTL calculation: right side is 0, left side is 100%
        const clickOffsetFromRight = rect.right - e.clientX
        const ratio = Math.max(0, Math.min(1, clickOffsetFromRight / rect.width))
        const timeAtCursor = ratio * this.totalDuration

        if (this.isDraggingStart) {
          this.setRange(timeAtCursor, this.endTime)
        } else if (this.isDraggingEnd) {
          this.setRange(this.startTime, timeAtCursor)
        }
      }

      const onMouseUp = () => {
        this.isDraggingStart = false
        this.isDraggingEnd = false
        document.removeEventListener('mousemove', onMouseMove)
        document.removeEventListener('mouseup', onMouseUp)
      }

      startHandle.addEventListener('mousedown', (e) => {
        e.preventDefault()
        this.isDraggingStart = true
        document.addEventListener('mousemove', onMouseMove)
        document.addEventListener('mouseup', onMouseUp)
      })

      endHandle.addEventListener('mousedown', (e) => {
        e.preventDefault()
        this.isDraggingEnd = true
        document.addEventListener('mousemove', onMouseMove)
        document.addEventListener('mouseup', onMouseUp)
      })
    }
  }
}
