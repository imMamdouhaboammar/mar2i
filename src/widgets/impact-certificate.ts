/**
 * Digital Impact Certificate & Archival Audit Receipt for Mar2i (مرئي)
 * Inspired by compressO's ImpactPop & verification receipt architecture.
 * Tailored for Saudi Ministry of Hajj & Umrah official records.
 */

export interface BatchImpactReport {
  batchId: string
  totalFiles: number
  totalOriginalBytes: number
  totalOutputBytes: number
  averageSsim: number
  rootSha256: string
  completedDate: string
  destinationFolder: string
  codecUsed: string
}

export interface ImpactCertificateOptions {
  container: HTMLElement
  report: BatchImpactReport
  onOpenFolder?: () => void
  onExportCsv?: () => void
  onCopyAuditHash?: () => void
}

export class ImpactCertificate {
  private container: HTMLElement
  private report: BatchImpactReport
  private options: ImpactCertificateOptions

  constructor(options: ImpactCertificateOptions) {
    this.options = options
    this.container = options.container
    this.report = options.report
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
    const orig = Math.max(1, this.report.totalOriginalBytes)
    const out = Math.min(orig, Math.max(0, this.report.totalOutputBytes))
    const saved = Math.max(0, orig - out)
    const savedPercent = Math.round((saved / orig) * 100)

    this.container.innerHTML = `
      <div class="impact-certificate-card" role="region" aria-label="سند الإنجاز والاعتماد الرقمي للدفعة" dir="rtl">
        <!-- Ministry Gold / Emerald Emblem Header -->
        <header class="certificate-header">
          <div class="certificate-seal">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
              <circle cx="12" cy="12" r="10"/>
              <path d="m9 12 2 2 4-4"/>
              <path d="M12 2v4M12 18v4M2 12h4M18 12h4"/>
            </svg>
          </div>
          <div>
            <div class="certificate-org">المملكة العربية السعودية · وزارة الحج والعمرة</div>
            <h2 class="certificate-title">سند الإنجاز والاعتماد الرقمي لمعالجة الدفعة (نموذج تجريبي غير معتمد)</h2>
            <div class="certificate-date">رقم الدفعة: ${this.report.batchId} · التاريخ: ${this.report.completedDate}</div>
          </div>
        </header>

        <!-- KPI Metrics Ribbon -->
        <div class="certificate-kpis-grid">
          <div class="kpi-box highlight-green">
            <span class="kpi-number">${this.formatBytes(saved)}</span>
            <span class="kpi-label">إجمالي السعة المحررة (${savedPercent}%)</span>
          </div>
          <div class="kpi-box">
            <span class="kpi-number">${this.report.totalFiles}</span>
            <span class="kpi-label">عدد ملفات الفيديو المعتمدة</span>
          </div>
          <div class="kpi-box">
            <span class="kpi-number">${(this.report.averageSsim * 100).toFixed(1)}%</span>
            <span class="kpi-label">مؤشر الجودة البصرية (SSIM: ${this.report.averageSsim.toFixed(3)})</span>
          </div>
          <div class="kpi-box">
            <span class="kpi-number">${this.report.codecUsed}</span>
            <span class="kpi-label">الترميز الحكومي المعتمد</span>
          </div>
        </div>

        <!-- SHA-256 Tamper-Evident Manifest Ledger Box -->
        <div class="certificate-hash-box">
          <div class="hash-box-head">
            <strong>بصمة السجل الأرشيفي الموحد (Root Merkle SHA-256):</strong>
            <button class="hash-copy-btn" type="button" id="btn-copy-root-hash" title="نسخ البصمة">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
              <span>نسخ كود الاعتماد</span>
            </button>
          </div>
          <code class="root-hash-code">${this.report.rootSha256}</code>
          <p class="hash-caption">قيمة توضيحية للواجهة فقط، وليست إثباتًا لسلامة الملفات أو اعتمادًا رسميًا. التحقق الفعلي يتطلب بصمة كاملة وبيانًا قابلاً لإعادة الفحص.</p>
        </div>

        <!-- Action Buttons -->
        <footer class="certificate-actions">
          <button class="cert-btn primary" type="button" id="cert-btn-folder">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>
            </svg>
            <span>فتح مجلد النتائج المحفوظة</span>
          </button>
          <button class="cert-btn outline" type="button" id="cert-btn-csv">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            <span>تصدير محضر الاعتماد الرسمي (CSV)</span>
          </button>
        </footer>
      </div>
    `

    this.attachEvents()
  }

  private attachEvents(): void {
    const copyBtn = this.container.querySelector<HTMLButtonElement>('#btn-copy-root-hash')
    copyBtn?.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(this.report.rootSha256)
        copyBtn.innerHTML = `
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          <span>تم النسخ!</span>
        `
        this.options.onCopyAuditHash?.()
        setTimeout(() => this.render(), 2000)
      } catch {}
    })

    const folderBtn = this.container.querySelector<HTMLButtonElement>('#cert-btn-folder')
    folderBtn?.addEventListener('click', () => {
      this.options.onOpenFolder?.()
    })

    const csvBtn = this.container.querySelector<HTMLButtonElement>('#cert-btn-csv')
    csvBtn?.addEventListener('click', () => {
      this.options.onExportCsv?.()
    })
  }
}
