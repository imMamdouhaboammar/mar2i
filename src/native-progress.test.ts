import './test-setup'
import { expect, test } from 'bun:test'

test('native progress pages expose every file in a 61-job batch', async () => {
  const jobs = Array.from({ length: 61 }, (_, index) => ({
    relative: `مقطع-${String(index + 1).padStart(3, '0')}.mp4`,
    status: 'queued',
    inputBytes: 1024,
    outputBytes: null,
    output: null,
    reason: null,
    quality: null,
    elapsedMs: null,
  }))
  let folderPick = 0
  let running = true
  const win = window as typeof window & {
    __TAURI_INTERNALS__?: { invoke: (command: string) => Promise<unknown> }
  }
  win.__TAURI_INTERNALS__ = {
    invoke: async (command) => {
      switch (command) {
        case 'plugin:dialog|open':
          return folderPick++ === 0 ? '/synthetic/source' : '/synthetic/output'
        case 'analyze_directory':
          return { total: 61, totalBytes: 62464, supported: 61, skipped: 0, details: [] }
        case 'start_batch':
        case 'control_batch':
          return null
        case 'batch_status':
          return {
            running, paused: false, cancelled: false, error: null,
            files: 61, completed: 1, skipped: 0, failed: 0, queued: 60, active: 0,
            inputBytes: 62464, outputBytes: 512, verifiedSavingsBytes: 512, jobs,
          }
        case 'detect_hardware_encoders':
          return { availableEncoders: [], recommendedH264: null, recommendedHevc: null }
        default:
          throw new Error(`Unexpected native call: ${command}`)
      }
    },
  }
  document.body.innerHTML = '<div id="app"></div>'
  await import(`./main?progress-isolation-${Date.now()}`)

  const click = async (selector: string) => {
    const item = document.querySelector<HTMLButtonElement>(selector)
    expect(item, `missing button ${selector}`).not.toBeNull()
    item?.click()
    await new Promise(resolve => setTimeout(resolve, 30))
  }

  try {
    await click('[data-action="select-source"]')
    await click('[data-action="select-output"]')
    await click('[data-action="analyze"]')
    await click('[data-action="start-preview"]')

    const seen = new Set<string>()
    for (const expectedRows of [30, 30, 1]) {
      const rows = Array.from(document.querySelectorAll('.progress-table tbody tr'))
      expect(rows).toHaveLength(expectedRows)
      for (const row of rows) {
        const label = row.querySelector('.name-cell span')?.textContent?.trim()
        expect(label).toBeTruthy()
        seen.add(label!)
      }
      if (expectedRows !== 1) await click('[data-progress-next]')
    }
    expect(seen.size).toBe(61)
    expect(seen.has('مقطع-061.mp4')).toBe(true)
  } finally {
    running = false
    await new Promise(resolve => setTimeout(resolve, 950))
  }
})
