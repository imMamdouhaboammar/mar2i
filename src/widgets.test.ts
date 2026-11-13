import './test-setup'
import { describe, expect, it } from 'bun:test'
import { DynamicIsland, type QueueItem } from './widgets/dynamic-island'
import { VideoTrimmer } from './widgets/video-trimmer'
import { AspectRatioStudio, ASPECT_PRESETS } from './widgets/aspect-ratio-studio'
import { StorageMeter } from './widgets/storage-meter'
import { ImpactCertificate } from './widgets/impact-certificate'
import { BatchActionBar } from './widgets/batch-action-bar'
import { CompareSlider } from './compare-slider'
import { getSampleProbeData } from './data'

describe('Widgets & Features Architecture Tests', () => {
  it('DynamicIsland mounts in collapsed state and expands correctly', () => {
    const container = document.createElement('div')

    const queue: QueueItem[] = [
      { id: '1', name: 'tawaf.mp4', originalSize: 1000, compressedSize: 500, codec: 'HEVC', status: 'completed', progress: 100 },
      { id: '2', name: 'hajj.mov', originalSize: 2000, codec: 'AV1', status: 'compressing', progress: 50 },
    ]

    const island = new DynamicIsland({
      container,
      initialExpanded: false,
      isProcessing: true,
      isPaused: false,
      progress: 75,
      keepAwakeActive: true,
      throughputRate: '4.2x',
      queue,
    })

    expect(container.querySelector('.island-pill')).not.toBeNull()
    expect(container.querySelector('.island-expanded-panel')).toBeNull()

    // Expand
    island.setExpanded(true)
    expect(container.querySelector('.island-expanded-panel')).not.toBeNull()
    expect(container.querySelectorAll('.island-queue-row').length).toBe(2)
  })

  it('VideoTrimmer clamps boundaries and formats timecodes accurately', () => {
    const container = document.createElement('div')
    let startVal = 0
    let endVal = 0

    const trimmer = new VideoTrimmer({
      container,
      fileName: 'crowd.mp4',
      totalDurationSeconds: 120,
      initialStartSeconds: 10,
      initialEndSeconds: 50,
      onTrimChange: (s, e) => {
        startVal = s
        endVal = e
      },
    })

    const initialRange = trimmer.getSelectedRange()
    expect(initialRange.start).toBe(10)
    expect(initialRange.end).toBe(50)
    expect(initialRange.duration).toBe(40)

    // Test programmatic range setting with clamping
    trimmer.setRange(-5, 150)
    const clampedRange = trimmer.getSelectedRange()
    expect(clampedRange.start).toBe(0)
    expect(clampedRange.end).toBe(120)
    expect(startVal).toBe(0)
    expect(endVal).toBe(120)
  })

  it('AspectRatioStudio rotates in 90 degree increments and toggles presets', () => {
    const container = document.createElement('div')
    let lastConfig: any = null

    const studio = new AspectRatioStudio({
      container,
      fileName: 'hajj_ceremony.mov',
      initialPresetId: '16-9',
      onApply: (cfg) => {
        lastConfig = cfg
      },
    })

    expect(ASPECT_PRESETS.length).toBeGreaterThanOrEqual(4)
    studio.rotateClockwise() // 90
    studio.rotateClockwise() // 180
    studio.rotateClockwise() // 270
    studio.rotateClockwise() // 360 -> 0

    studio.toggleFlip()
    studio.setPreset('9-16')

    const applyBtn = container.querySelector<HTMLButtonElement>('#btn-apply-aspect')
    applyBtn?.click()

    expect(lastConfig).not.toBeNull()
    expect(lastConfig.presetId).toBe('9-16')
    expect(lastConfig.flipped).toBe(true)
    expect(lastConfig.rotation).toBe(0)
  })

  it('StorageMeter calculates liberated capacity segments accurately', () => {
    const container = document.createElement('div')
    new StorageMeter({
      container,
      forecast: {
        sourceBytes: 100 * 1024 * 1024 * 1024, // 100 GB
        estimatedOutputBytes: 25 * 1024 * 1024 * 1024, // 25 GB (75% savings)
        codecName: 'HEVC',
        mode: 'publish',
      },
    })

    const card = container.querySelector('.storage-meter-card')
    expect(card).not.toBeNull()
    const savingBadge = container.querySelector('.badge-saving-num')
    expect(savingBadge?.textContent).toBe('75%')
  })

  it('ImpactCertificate formats Ministry KPIs and Merkle root hash', () => {
    const container = document.createElement('div')
    new ImpactCertificate({
      container,
      report: {
        batchId: 'BATCH-TEST-01',
        totalFiles: 42,
        totalOriginalBytes: 100 * 1024 * 1024 * 1024,
        totalOutputBytes: 30 * 1024 * 1024 * 1024,
        averageSsim: 0.994,
        rootSha256: 'fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210',
        completedDate: '2026-11-11',
        destinationFolder: '/Volumes/Archive/Output',
        codecUsed: 'HEVC 10-bit',
      },
    })

    const title = container.querySelector('.certificate-title')
    expect(title?.textContent).toContain('سند الإنجاز والاعتماد الرقمي')
    const code = container.querySelector('.root-hash-code')
    expect(code?.textContent).toContain('fedcba9876543210')
  })

  it('BatchActionBar manages multi-file selections and triggers batch handlers', () => {
    const container = document.createElement('div')
    let appliedCodec = ''

    const bar = new BatchActionBar({
      container,
      selectedCount: 5,
      totalCount: 20,
      onApplyCodec: (cid) => {
        appliedCodec = cid
      },
    })

    expect(container.querySelector('.batch-action-bar-floating')).not.toBeNull()
    expect(container.querySelector('.batch-bar-badge')?.textContent).toBe('5')

    const hevcBtn = container.querySelector<HTMLButtonElement>('[data-batch-action="codec-hevc"]')
    hevcBtn?.click()
    expect(appliedCodec).toBe('hevc')

    // Disappears when count reaches 0
    bar.updateCount(0)
    expect(container.querySelector('.batch-action-bar-floating')).toBeNull()
  })

  it('CompareSlider initialises and changes position', () => {
    const container = document.createElement('div')

    const slider = new CompareSlider({
      container,
      beforeImage: 'data:image/svg+xml;utf8,<svg></svg>',
      afterImage: 'data:image/svg+xml;utf8,<svg></svg>',
      initialPosition: 60,
    })

    expect(container.querySelector('.compare-handle')).not.toBeNull()
    slider.setPosition(80)
    const handle = container.querySelector('.compare-handle')
    expect(handle?.getAttribute('aria-valuenow')).toBe('80')
  })

  it('getSampleProbeData returns complete forensic media metadata', () => {
    const movData = getSampleProbeData('ceremony.mov')
    expect(movData.formatName).toContain('mov')
    expect(movData.fps).toBe(59.94)
    expect(movData.audioChannels).toBe(2)
    expect(movData.sparseSha256).toBeDefined()
    expect(movData.preservationClass).toBe('Master Permanent Archive')

    const mp4Data = getSampleProbeData('guide.mp4')
    expect(mp4Data.formatName).toContain('mp4')
    expect(mp4Data.preservationClass).toBe('Standard Media')
  })
})
