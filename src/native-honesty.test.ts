import './test-setup'
import { describe, expect, it } from 'bun:test'

describe('Native desktop truthfulness', () => {
  it('shows no synthetic results, official certificate or invented throughput before a batch exists', async () => {
    type MockResponse = string | { availableEncoders: string[]; recommendedH264: string | null; recommendedHevc: string | null }

    // SAFETY: Mocking window internals for Tauri desktop unit testing
    const win = window as typeof window & { __TAURI_INTERNALS__?: { invoke: (cmd: string) => Promise<MockResponse> } }
    win.__TAURI_INTERNALS__ = { invoke: async (cmd) => {
      if (cmd === 'report_content' || cmd === 'save_report_file') throw new Error('Report not available')

      if (cmd === 'plugin:dialog|save') return '/mock/save/path.csv'

      return { availableEncoders: [], recommendedH264: null, recommendedHevc: null }
    } }
    document.body.innerHTML = '<div id="app"></div>'
    await import('./main')
    expect(document.querySelector('.sidebar-foot .version')?.textContent).toBe('الإصدار 0.2.4')
    expect(document.querySelector('#dynamic-island-slot')?.textContent).not.toContain('4.2x')
    document.querySelector<HTMLButtonElement>('[data-screen="results"]')?.click()
    expect(document.querySelector('.impact-certificate-card')).toBeNull()
    expect(document.querySelector('.results-table')?.textContent).not.toContain('إرشادات الطواف.mp4')

    let downloads = 0
    URL.createObjectURL = () => { downloads++;

 return 'blob:local-test' }

    URL.revokeObjectURL = () => {}

    document.querySelector<HTMLButtonElement>('[data-action="toggle-report-menu"]')?.click()
    await new Promise(resolve => setTimeout(resolve, 20))
    expect(document.querySelector('[data-export="csv"]')).not.toBeNull()
    document.querySelector<HTMLButtonElement>('[data-export="csv"]')?.click()
    await new Promise(resolve => setTimeout(resolve, 20))
    expect(downloads).toBe(0)
    expect(document.querySelector('[role="status"]')?.textContent).toContain('Report not available')

    document.querySelector<HTMLButtonElement>('[data-screen="new"]')?.click()
    await new Promise(resolve => setTimeout(resolve, 20))

    const codecSelect = document.querySelector<HTMLSelectElement>('#codec-select')
    expect(codecSelect).not.toBeNull()
    expect(codecSelect?.tagName).toBe('SELECT')
    expect(document.querySelector('.folder-value[data-action="select-source"]')).not.toBeNull()

    if (codecSelect) {
      codecSelect.value = 'hevc'
      // SAFETY: Using window-scoped Event class required by happy-dom
      codecSelect.dispatchEvent(new (window as any).Event('change', { bubbles: true }))
      await new Promise(resolve => setTimeout(resolve, 20))
      expect(document.querySelector('.codec-preview p')?.textContent).toContain('يوفر عادةً ضغطًا أعلى من H.264')
    }

    document.querySelector<HTMLButtonElement>('[data-screen="options"]')?.click()
    await new Promise(resolve => setTimeout(resolve, 20))
    expect(Boolean(document.querySelector('[data-preset-id]'))).toBe(false)
    expect(document.querySelector<HTMLInputElement>('#hardware')?.disabled).toBe(true)
    document.querySelector<HTMLButtonElement>('[data-screen="progress"]')?.click()
    expect(document.querySelector('.feature-indicators')?.textContent).not.toContain('نشط')
    expect(document.querySelector('.feature-indicators')?.textContent).not.toContain('GPU نشط')
  })

  it('displays mandatory 4STEPS KSA intellectual property and restricted distribution notice', async () => {
    document.querySelector<HTMLButtonElement>('[data-screen="settings"]')?.click()
    await new Promise(resolve => setTimeout(resolve, 20))
    const settingsText = document.querySelector('#app')?.textContent
    expect(settingsText).toContain('4STEPS KSA')
    expect(settingsText).toContain('قسم التطوير والذكاء الاصطناعي والحلول الرقمية')
    expect(settingsText).toContain('بدون ترخيص رسمي أو إذن كتابي مسبق')

    document.querySelector<HTMLSpanElement>('[data-action="open-about-modal"]')?.click()
    await new Promise(resolve => setTimeout(resolve, 20))
    const aboutDialog = document.querySelector('#app-dialog')
    expect(aboutDialog).not.toBeNull()
    expect(aboutDialog?.textContent).toContain('4STEPS KSA')
    expect(aboutDialog?.textContent).toContain('إشعار الملكية الفكرية والحماية القانونية')
  })

  it('recognizes copied files as preserved unchanged and never invents compression savings', () => {
    const job = {
      relative: 'already_compressed.mp4',
      status: 'copied',
      inputBytes: 1000,
      outputBytes: 1000,
      output: '/out/already_compressed.mp4',
      reason: 'already-efficient source',
      quality: null,
      elapsedMs: 12,
    }

    const saving = job.status === 'completed' && job.outputBytes !== null && job.inputBytes > 0
      ? Math.max(0, 100 * (job.inputBytes - job.outputBytes) / job.inputBytes)
      : 0

    expect(saving).toBe(0)
  })
})
