import { describe, expect, test } from 'bun:test'
import { codecs, escapeHtml, getCodec, progressPageWindow, recommendedCodec, reportCsv, safeCsvCell, savingsPercent } from './data'

describe('compression choice copy', () => {
  test('all six choices have distinct explanatory trade-offs', () => {
    expect(codecs).toHaveLength(6)
    expect(new Set(codecs.map((codec) => codec.id)).size).toBe(6)
    for (const codec of codecs) {
      expect(codec.description.length).toBeGreaterThan(20)
      expect(codec.caveat.length).toBeGreaterThan(20)
      expect(codec.recommendation.length).toBeGreaterThan(20)
    }
  })

  test('archiving recommendation is not a publishing codec', () => {
    expect(recommendedCodec('archive')).toBe('ffv1')
    expect(recommendedCodec('publish')).toBe('h264')
    expect(getCodec('ffv1').bestFor).toBe('archive')
  })
})

describe('report calculations and safety', () => {
  test('savings can be negative and do not invent savings for zero-sized source', () => {
    expect(savingsPercent(100, 40)).toBe(60)
    expect(savingsPercent(100, 120)).toBe(-20)
    expect(savingsPercent(0, 0)).toBe(0)
  })

  test('CSV escapes quotes and blocks spreadsheet formulas', () => {
    expect(safeCsvCell('=1+2')).toBe('"\'=1+2"')
    expect(safeCsvCell('value"x')).toBe('"value""x"')
    expect(reportCsv([{ name: '=HYPERLINK("evil")', before: 4, after: 2, status: 'complete', codec: 'H.264' }])).toContain("'=HYPERLINK")
  })

  test('file names cannot inject HTML markup', () => {
    expect(escapeHtml('<img src="x">')).toBe('&lt;img src=&quot;x&quot;&gt;')
  })
})

describe('native progress pagination', () => {
  test('keeps every job reachable in stable pages of thirty', () => {
    expect(progressPageWindow(61, 0)).toEqual({ page: 0, pages: 3, start: 0, end: 30 })
    expect(progressPageWindow(61, 1)).toEqual({ page: 1, pages: 3, start: 30, end: 60 })
    expect(progressPageWindow(61, 2)).toEqual({ page: 2, pages: 3, start: 60, end: 61 })
  })

  test('bounds invalid page selections without losing small batches', () => {
    expect(progressPageWindow(61, -1)).toEqual({ page: 0, pages: 3, start: 0, end: 30 })
    expect(progressPageWindow(61, 99)).toEqual({ page: 2, pages: 3, start: 60, end: 61 })
    expect(progressPageWindow(0, 9)).toEqual({ page: 0, pages: 1, start: 0, end: 0 })
    expect(progressPageWindow(30, 1)).toEqual({ page: 0, pages: 1, start: 0, end: 30 })
  })
})
