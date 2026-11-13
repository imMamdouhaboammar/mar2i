/**
 * Floating Multi-File Batch Action Bar Widget for Mar2i (مرئي)
 * Inspired by compressO's CustomizeMediaOnBatchActions architecture.
 * Tailored for Saudi Ministry of Hajj & Umrah bulk operations.
 */

export interface BatchActionBarOptions {
  container: HTMLElement
  selectedCount: number
  totalCount: number
  onApplyCodec?: (codecId: string) => void
  onApplyAspect?: (aspectId: string) => void
  onTrimBatch?: () => void
  onExportSelected?: () => void
  onClearSelection?: () => void
}

export class BatchActionBar {
  private container: HTMLElement
  private selectedCount: number
  private totalCount: number
  private options: BatchActionBarOptions

  constructor(options: BatchActionBarOptions) {
    this.options = options
    this.container = options.container
    this.selectedCount = options.selectedCount
    this.totalCount = options.totalCount
    this.render()
  }

  public updateCount(selectedCount: number, totalCount?: number): void {
    this.selectedCount = selectedCount

    if (totalCount !== undefined) this.totalCount = totalCount
    this.render()
  }

  private render(): void {
    if (this.selectedCount <= 0) {
      this.container.innerHTML = ''

      return
    }

    this.container.innerHTML = `
      <div class="batch-action-bar-floating" role="toolbar" aria-label="شريط الإجراءات المجمعة" dir="rtl">
        <div class="batch-bar-counter">
          <span class="batch-bar-badge">${this.selectedCount}</span>
          <span>فيديو محدد من أصل ${this.totalCount}</span>
        </div>

        <div class="batch-bar-actions">
          <button class="batch-bar-btn" type="button" data-batch-action="codec-hevc" title="تطبيق ترميز HEVC الموفر على المحددة">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polygon points="12 2 2 7 12 12 22 7 12 2"/>
              <polyline points="2 17 12 22 22 17"/>
              <polyline points="2 12 12 17 22 12"/>
            </svg>
            <span>ترميز HEVC</span>
          </button>

          <button class="batch-bar-btn" type="button" data-batch-action="aspect-mobile" title="ملاءمة الأبعاد لمنصات الجوال (9:16)">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="5" y="2" width="14" height="20" rx="2" ry="2"/>
              <line x1="12" y1="18" x2="12.01" y2="18"/>
            </svg>
            <span>أبعاد الجوال (9:16)</span>
          </button>

          <button class="batch-bar-btn" type="button" data-batch-action="trim" title="اقتصاص المقدمة والخاتمة لجميع المحددة">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="6" cy="6" r="3"/>
              <circle cx="6" cy="18" r="3"/>
              <line x1="20" y1="4" x2="8.12" y2="15.88"/>
              <line x1="14.47" y1="14.48" x2="20" y2="20"/>
              <line x1="8.12" y1="8.12" x2="12" y2="12"/>
            </svg>
            <span>تشذيب مجمع</span>
          </button>

          <button class="batch-bar-btn" type="button" data-batch-action="export" title="تصدير كشف بالملفات المحددة فقط">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            <span>تقرير المحددة</span>
          </button>

          <button class="batch-bar-btn clear-btn" type="button" data-batch-action="clear" title="إلغاء التحديد">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
            <span>إلغاء</span>
          </button>
        </div>
      </div>
    `

    this.attachEvents()
  }

  private attachEvents(): void {
    const hevcBtn = this.container.querySelector<HTMLButtonElement>('[data-batch-action="codec-hevc"]')
    hevcBtn?.addEventListener('click', () => this.options.onApplyCodec?.('hevc'))

    const aspectBtn = this.container.querySelector<HTMLButtonElement>('[data-batch-action="aspect-mobile"]')
    aspectBtn?.addEventListener('click', () => this.options.onApplyAspect?.('9-16'))

    const trimBtn = this.container.querySelector<HTMLButtonElement>('[data-batch-action="trim"]')
    trimBtn?.addEventListener('click', () => this.options.onTrimBatch?.())

    const exportBtn = this.container.querySelector<HTMLButtonElement>('[data-batch-action="export"]')
    exportBtn?.addEventListener('click', () => this.options.onExportSelected?.())

    const clearBtn = this.container.querySelector<HTMLButtonElement>('[data-batch-action="clear"]')
    clearBtn?.addEventListener('click', () => this.options.onClearSelection?.())
  }
}
