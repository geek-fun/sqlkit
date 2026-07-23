import type { DagreRel, DagreTable } from '@/components/er-diagram/graph-layout'
/**
 * @jest-environment node
 */
import {
  computeCanvasSize,
  computeDagreLayout,
  fitToScreen,
} from '@/components/er-diagram/graph-layout'

describe('computeDagreLayout', () => {
  it('returns positions for A→B with finite coordinates, B below A', () => {
    const tables: DagreTable[] = [
      { name: 'A', width: 220, height: 100 },
      { name: 'B', width: 220, height: 100 },
    ]
    const rels: DagreRel[] = [
      { sourceTable: 'A', targetTable: 'B' },
    ]

    const positions = computeDagreLayout(tables, rels)
    const keys = Object.keys(positions)

    expect(keys).toContain('A')
    expect(keys).toContain('B')
    expect(Number.isFinite(positions.A.x)).toBe(true)
    expect(Number.isFinite(positions.A.y)).toBe(true)
    expect(Number.isFinite(positions.B.x)).toBe(true)
    expect(Number.isFinite(positions.B.y)).toBe(true)
    expect(positions.B.y).toBeGreaterThan(positions.A.y)
  })
})

describe('computeCanvasSize', () => {
  it('computes size from table positions with padding', () => {
    const positions: Record<string, { x: number, y: number }> = {
      A: { x: 110, y: 80 },
      B: { x: 410, y: 160 },
    }
    const getHeight = (name: string) => name === 'A' ? 100 : 100
    const result = computeCanvasSize(positions, 220, getHeight, 80)

    // A right edge = 110 + 110 + 80 = 300
    // B right edge = 410 + 110 + 80 = 600
    // B bottom edge = 160 + 50 + 80 = 290
    // min 960 x 540
    expect(result.width).toBe(960)
    expect(result.height).toBe(540)
  })

  it('expands beyond minimum when tables are large', () => {
    const positions: Record<string, { x: number, y: number }> = {
      Big: { x: 1000, y: 800 },
    }
    const getHeight = () => 200
    const result = computeCanvasSize(positions, 220, getHeight, 50)

    // right edge = 1000 + 110 + 50 = 1160
    // bottom edge = 800 + 100 + 50 = 950
    expect(result.width).toBe(1160)
    expect(result.height).toBe(950)
  })

  it('defaults to 960x540 for empty positions', () => {
    const result = computeCanvasSize({}, 220, () => 0, 80)
    expect(result.width).toBe(960)
    expect(result.height).toBe(540)
  })
})

describe('fitToScreen', () => {
  it('zooms in (zoom >= 1) when content is smaller than viewport', () => {
    const result = fitToScreen({ width: 100, height: 100 }, 800, 600)
    expect(result.zoom).toBe(3)
    expect(Number.isFinite(result.scrollX)).toBe(true)
    expect(Number.isFinite(result.scrollY)).toBe(true)
  })

  it('zooms out (zoom < 1) when content is larger than viewport', () => {
    const result = fitToScreen({ width: 2000, height: 1500 }, 800, 600)
    expect(result.zoom).toBeCloseTo(0.4, 5)
    expect(Number.isFinite(result.scrollX)).toBe(true)
    expect(Number.isFinite(result.scrollY)).toBe(true)
  })

  it('centers content in viewport', () => {
    const result = fitToScreen({ width: 400, height: 300 }, 1000, 800)
    expect(result.zoom).toBe(2.5)
    expect(result.scrollX).toBe(0)
    expect(result.scrollY).toBe(25)
  })

  it('caps zoom between 0.1 and 3', () => {
    const r1 = fitToScreen({ width: 10, height: 10 }, 10000, 10000)
    expect(r1.zoom).toBe(3)
    const r2 = fitToScreen({ width: 100000, height: 100000 }, 100, 100)
    expect(r2.zoom).toBe(0.1)
  })
})
