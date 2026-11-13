/**
 * Live Predictive Storage Meter & Capacity Bar Widget for Mar2i (مرئي)
 * Inspired by compressO's empirical size estimation architecture.
 * Clean-room implementation for Saudi Ministry of Hajj & Umrah storage planning.
 */

export interface StorageForecast {
  sourceBytes: number
  estimatedOutputBytes: number
  codecName: string
  mode: 'publish' | 'archive'
}

export interface StorageMeterOptions {
  container: HTMLElement
  forecast: StorageForecast
  onInspectDetails?: () => void
}

export class StorageMeter {
  private container: HTMLElement
  private forecast: StorageForecast

  constructor(options: StorageMeterOptions) {
    this.container = options.container
    this.forecast = options.forecast
    this.render()
  }

  public update(forecast: StorageForecast): void {
    this.forecast = forecast
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
    const src = Math.max(1, this.forecast.sourceBytes)
    const out = Math.min(src, Math.max(0, this.forecast.estimatedOutputBytes))
    const saved = Math.max(0, src - out)
    const savedPercent = Math.round((saved / src) * 100)
    const outputPercent = 100 - savedPercent

    // Equivalent metric approximations
    const equivalentSsds = (saved / (1024 * 1024 * 1024 * 32)).toFixed(1) // 32GB units
    const egressSpeedup = ((src / Math.max(1, out))).toFixed(1)

    this.container.innerHTML = `
      <div class="storage-meter-card" role="region" aria-label="مسطرة توقع السعة التخزينية المباشرة" dir="rtl">
        <header class="storage-meter-head">
          <div class="meter-title-group">
            <div class="meter-icon-pill">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="2" y="2" width="20" height="8" rx="2" ry="2"/>
                <rect x="2" y="14" width="20" height="8" rx="2" ry="2"/>
                <line x1="6" y1="6" x2="6.01" y2="6"/>
                <line x1="6" y1="18" x2="6.01" y2="18"/>
              </svg>
            </div>
            <div>
              <strong>التوفير المتوقع في السعة التخزينية</strong>
              <div class="meter-subtitle">بناءً على قياس عينات المصدر الفعلية وخوارزمية (${this.forecast.codecName})</div>
            </div>
          </div>
          <div class="meter-stat-badge">
            <span class="badge-saving-num">${savedPercent}%</span>
            <span class="badge-saving-label">توفير متوقع</span>
          </div>
        </header>

        <!-- Multi-Segment Visual Capacity Bar -->
        <div class="capacity-bar-wrapper">
          <div class="capacity-bar-track">
            <div class="capacity-segment-output" style="width: ${outputPercent}%;" title="الحجم بعد الضغط: ${this.formatBytes(out)} (${outputPercent}%)">
              <span class="segment-label">${this.formatBytes(out)}</span>
            </div>
            <div class="capacity-segment-saved" style="width: ${savedPercent}%;" title="المساحة المحررة: ${this.formatBytes(saved)} (${savedPercent}%)">
              <span class="segment-label">وفر ${this.formatBytes(saved)}</span>
            </div>
          </div>
          <div class="capacity-bar-legend">
            <span class="legend-item"><span class="legend-dot dot-output"></span>الحجم الناتج المتوقع (${this.formatBytes(out)})</span>
            <span class="legend-item"><span class="legend-dot dot-saved"></span>المساحة المحررة الموفرة (${this.formatBytes(saved)})</span>
            <span class="legend-item"><span class="legend-dot dot-source"></span>إجمالي المصدر (${this.formatBytes(src)})</span>
          </div>
        </div>

        <!-- Equivalent Government Impact Metrics -->
        <div class="impact-metrics-grid">
          <div class="impact-metric-pill">
            <span class="impact-metric-icon">💾</span>
            <div class="impact-metric-text">
              <strong>${equivalentSsds} وحدة تخزين</strong>
              <small>سعة وسائط محررة لخوادم الوزارة</small>
            </div>
          </div>
          <div class="impact-metric-pill">
            <span class="impact-metric-icon">⚡</span>
            <div class="impact-metric-text">
              <strong>تسريع الرفع بمعدل ${egressSpeedup}x</strong>
              <small>تقليص زمن البث والنقل الشبكي</small>
            </div>
          </div>
        </div>
      </div>
    `
  }
}
