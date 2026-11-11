/**
 * Dynamic Island & Drop Zone Widget for Mar2i (مرئي)
 * Inspired by compressO's DropZoneIsland architecture.
 * Clean-room implementation tailored for Saudi Ministry of Hajj & Umrah bulk operations.
 */

export interface QueueItem {
  id: string
  name: string
  originalSize: number
  compressedSize?: number
  codec: string
  status: 'waiting' | 'compressing' | 'completed' | 'failed' | 'cancelled'
  progress: number
}

export interface DynamicIslandOptions {
  container: HTMLElement
  initialExpanded?: boolean
  isProcessing: boolean
  isPaused: boolean
  progress: number
  keepAwakeActive: boolean
  throughputRate: string
  queue: QueueItem[]
  onToggleExpand?: (expanded: boolean) => void
  onPauseToggle?: () => void
  onCancelBatch?: () => void
  onClearCompleted?: () => void
  onFilesDropped?: (files: File[]) => void
  onItemAction?: (itemId: string, action: 'cancel' | 'retry' | 'inspect') => void
}

export class DynamicIsland {
  private container: HTMLElement
  private isExpanded: boolean
  private isProcessing: boolean
  private isPaused: boolean
  private progress: number
  private keepAwakeActive: boolean
  private throughputRate: string
  private queue: QueueItem[]
  private options: DynamicIslandOptions

  constructor(options: DynamicIslandOptions) {
    this.options = options
    this.container = options.container
    this.isExpanded = options.initialExpanded ?? false
    this.isProcessing = options.isProcessing
    this.isPaused = options.isPaused
    this.progress = options.progress
    this.keepAwakeActive = options.keepAwakeActive
    this.throughputRate = options.throughputRate
    this.queue = [...options.queue]
    this.render()
  }

  public update(partial: Partial<DynamicIslandOptions>): void {
    if (partial.isProcessing !== undefined) this.isProcessing = partial.isProcessing
    if (partial.isPaused !== undefined) this.isPaused = partial.isPaused
    if (partial.progress !== undefined) this.progress = partial.progress
    if (partial.keepAwakeActive !== undefined) this.keepAwakeActive = partial.keepAwakeActive
    if (partial.throughputRate !== undefined) this.throughputRate = partial.throughputRate
    if (partial.queue !== undefined) this.queue = [...partial.queue]
    if (partial.initialExpanded !== undefined) this.isExpanded = partial.initialExpanded
    this.render()
  }

  public setExpanded(expanded: boolean): void {
    if (this.isExpanded === expanded) return
    this.isExpanded = expanded
    this.options.onToggleExpand?.(this.isExpanded)
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

  private render(): void {
    const completedCount = this.queue.filter((q) => q.status === 'completed').length
    const totalCount = this.queue.length
    const radius = 14
    const circumference = 2 * Math.PI * radius
    const strokeDashoffset = circumference - (this.progress / 100) * circumference

    this.container.innerHTML = `
      <div class="dynamic-island-root ${this.isExpanded ? 'is-expanded' : 'is-collapsed'}"
           role="region"
           aria-label="جزيرة العمليات التفاعلية"
           dir="rtl">

        <!-- Collapsed Pill View -->
        <div class="island-pill"
             tabindex="0"
             role="button"
             aria-expanded="${this.isExpanded}"
             aria-label="تبديل فتح وإغلاق جزيرة العمليات">
          <div class="island-pill-inner">
            <div class="island-pill-progress">
              <svg class="island-radial-svg" width="34" height="34" viewBox="0 0 34 34">
                <circle class="radial-track" cx="17" cy="17" r="${radius}" fill="none" stroke="currentColor" stroke-width="3" opacity="0.15" />
                <circle class="radial-fill" cx="17" cy="17" r="${radius}" fill="none" stroke="currentColor" stroke-width="3"
                        stroke-dasharray="${circumference}"
                        stroke-dashoffset="${strokeDashoffset}"
                        stroke-linecap="round" />
              </svg>
              <span class="radial-text">${Math.round(this.progress)}%</span>
            </div>

            <div class="island-pill-info">
              <span class="island-pill-title">
                ${this.isProcessing ? (this.isPaused ? 'الدفعة متوقفة مؤقتًا' : `معالجة ${completedCount} من ${totalCount} فيديو`) : 'جاهز للإسقاط والتشغيل'}
              </span>
              <span class="island-pill-sub">
                ${this.throughputRate ? `السرعة: ${this.throughputRate}` : 'خامل'}
              </span>
            </div>

            <div class="island-pill-badges">
              <span class="island-shield-badge ${this.keepAwakeActive ? 'is-active' : ''}"
                    title="${this.keepAwakeActive ? 'مانع سكون النظام نشط (KeepAwake)' : 'مانع السكون غير مفعل'}">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                </svg>
                <span class="shield-pulse"></span>
              </span>
              <span class="island-chevron-icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="m6 9 6 6 6-6"/>
                </svg>
              </span>
            </div>
          </div>
        </div>

        <!-- Expanded Drawer Panel -->
        ${this.isExpanded ? `
          <div class="island-expanded-panel">
            <header class="island-header">
              <div class="island-header-title">
                <div class="island-icon-box">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polygon points="12 2 2 7 12 12 22 7 12 2"/>
                    <polyline points="2 17 12 22 22 17"/>
                    <polyline points="2 12 12 17 22 12"/>
                  </svg>
                </div>
                <div>
                  <strong>مركز إدارة الدفعة والمهام</strong>
                  <div class="island-subtitle">${this.isPaused ? 'المعالجة متوقفة' : 'معالجة جارية باستخدام خوارزميات الضغط الحكومية'}</div>
                </div>
              </div>
              <button class="island-close-btn" type="button" aria-label="طي الجزيرة">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18"/>
                  <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </header>

            <!-- Drop Zone Strip -->
            <div class="island-drop-strip" role="region" aria-label="منطقة الإسقاط السريع">
              <div class="island-drop-content">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                  <polyline points="17 8 12 3 7 8"/>
                  <line x1="12" y1="3" x2="12" y2="15"/>
                </svg>
                <span>اسحب وأفلت لقطات فيديو جديدة هنا لإضافتها مباشرة للدفعة</span>
              </div>
            </div>

            <!-- Queue Items List -->
            <div class="island-queue-list" role="list">
              ${this.queue.length === 0 ? `
                <div class="island-queue-empty">لا توجد ملفات حالية في قائمة الانتظار</div>
              ` : this.queue.map((item) => `
                <div class="island-queue-row status-${item.status}" role="listitem">
                  <div class="queue-row-info">
                    <span class="queue-row-name" title="${item.name}">${item.name}</span>
                    <div class="queue-row-meta">
                      <span class="queue-codec-tag">${item.codec}</span>
                      <span>${this.formatBytes(item.originalSize)}</span>
                      ${item.compressedSize ? `<span class="queue-savings">← ${this.formatBytes(item.compressedSize)}</span>` : ''}
                    </div>
                  </div>
                  <div class="queue-row-status">
                    <span class="status-pill pill-${item.status}">
                      ${item.status === 'compressing' ? `${Math.round(item.progress)}%` :
                        item.status === 'completed' ? 'مكتمل' :
                        item.status === 'waiting' ? 'منتظر' : 'متوقف'}
                    </span>
                    <button class="queue-action-btn" type="button" data-item-id="${item.id}" data-action="inspect" title="فحص التفاصيل">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <line x1="12" y1="16" x2="12" y2="12"/>
                        <line x1="12" y1="8" x2="12.01" y2="8"/>
                      </svg>
                    </button>
                  </div>
                </div>
              `).join('')}
            </div>

            <!-- Footer Actions -->
            <footer class="island-footer">
              <button class="island-btn primary" type="button" data-island-action="pause">
                ${this.isPaused ? `
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                  <span>استئناف المعالجة</span>
                ` : `
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
                  <span>إيقاف مؤقت</span>
                `}
              </button>
              <button class="island-btn outline" type="button" data-island-action="clear">
                مسح المنتهي
              </button>
              <button class="island-btn danger" type="button" data-island-action="cancel">
                إلغاء الدفعة
              </button>
            </footer>
          </div>
        ` : ''}
      </div>
    `

    this.attachEvents()
  }

  private attachEvents(): void {
    const pill = this.container.querySelector<HTMLElement>('.island-pill')
    pill?.addEventListener('click', () => {
      this.setExpanded(!this.isExpanded)
    })
    pill?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault()
        this.setExpanded(!this.isExpanded)
      }
    })

    const closeBtn = this.container.querySelector<HTMLButtonElement>('.island-close-btn')
    closeBtn?.addEventListener('click', (e) => {
      e.stopPropagation()
      this.setExpanded(false)
    })

    const pauseBtn = this.container.querySelector<HTMLButtonElement>('[data-island-action="pause"]')
    pauseBtn?.addEventListener('click', () => {
      this.options.onPauseToggle?.()
    })

    const clearBtn = this.container.querySelector<HTMLButtonElement>('[data-island-action="clear"]')
    clearBtn?.addEventListener('click', () => {
      this.options.onClearCompleted?.()
    })

    const cancelBtn = this.container.querySelector<HTMLButtonElement>('[data-island-action="cancel"]')
    cancelBtn?.addEventListener('click', () => {
      this.options.onCancelBatch?.()
    })

    const inspectBtns = this.container.querySelectorAll<HTMLButtonElement>('[data-action="inspect"]')
    inspectBtns.forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation()
        const id = btn.getAttribute('data-item-id')
        if (id) this.options.onItemAction?.(id, 'inspect')
      })
    })

    // Drag & Drop handling on the Island
    const dropStrip = this.container.querySelector<HTMLElement>('.island-drop-strip')
    if (dropStrip) {
      dropStrip.addEventListener('dragover', (e) => {
        e.preventDefault()
        dropStrip.classList.add('drag-active')
      })
      dropStrip.addEventListener('dragleave', () => {
        dropStrip.classList.remove('drag-active')
      })
      dropStrip.addEventListener('drop', (e) => {
        e.preventDefault()
        dropStrip.classList.remove('drag-active')
        if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
          const files = Array.from(e.dataTransfer.files)
          this.options.onFilesDropped?.(files)
        }
      })
    }
  }
}
