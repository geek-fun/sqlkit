import type { DagreRel, DagreTable } from '@/components/er-diagram/graph-layout'
/**
 * @jest-environment node
 */
import {
  computeBoundingBox,
  computeCanvasSize,
  computeDagreLayout,

  fitToScreen,
} from '@/components/er-diagram/graph-layout'

describe('computeDagreLayout', () => {
  it('returns Map with both entries and finite coordinates for A→B', () => {
    const tables: DagreTable[] = [
      { name: 'A', width: 220, height: 100 },
      { name: 'B', width: 220, height: 100 },
    ]
    const rels: DagreRel[] = [
      { sourceTable: 'A', targetTable: 'B' },
    ]

    const positions = computeDagreLayout(tables, rels, 'TB')

    expect(positions.size).toBe(2)
    expect(positions.has('A')).toBe(true)
    expect(positions.has('B')).toBe(true)

    const posA = positions.get('A')!
    const posB = positions.get('B')!

    expect(Number.isFinite(posA.x)).toBe(true)
    expect(Number.isFinite(posA.y)).toBe(true)
    expect(Number.isFinite(posB.x)).toBe(true)
    expect(Number.isFinite(posB.y)).toBe(true)

    // In TB layout, B (child) should be below A (parent)
    expect(posB.y).toBeGreaterThan(posA.y)
  })

  it('supports LR direction with B to the right of A', () => {
    const tables: DagreTable[] = [
      { name: 'A', width: 220, height: 100 },
      { name: 'B', width: 220, height: 100 },
    ]
    const rels: DagreRel[] = [
      { sourceTable: 'A', targetTable: 'B' },
    ]

    const positions = computeDagreLayout(tables, rels, 'LR')

    expect(positions.size).toBe(2)
    const posA = positions.get('A')!
    const posB = positions.get('B')!
    expect(posB.x).toBeGreaterThan(posA.x)
  })
})

describe('computeBoundingBox', () => {
  it('returns expected min/max for known positions', () => {
    const positions = new Map([
      ['A', { x: 110, y: 80 }],
      ['B', { x: 410, y: 160 }],
    ])
    const sizes = new Map([
      ['A', { width: 220, height: 100 }],
      ['B', { width: 220, height: 100 }],
    ])

    const bbox = computeBoundingBox(positions, sizes)

    // A: left=110-110=0, top=80-50=30, right=110+110=220, bottom=80+50=130
    // B: left=410-110=300, top=160-50=110, right=410+110=520, bottom=160+50=210
    expect(bbox.minX).toBe(0)
    expect(bbox.minY).toBe(30)
    expect(bbox.maxX).toBe(520)
    expect(bbox.maxY).toBe(210)
  })

  it('returns zeros for empty inputs', () => {
    expect(computeBoundingBox(new Map(), new Map())).toEqual({
      minX: 0,
      minY: 0,
      maxX: 0,
      maxY: 0,
    })
  })

  it('skips entries with missing sizes', () => {
    const positions = new Map([
      ['A', { x: 100, y: 100 }],
      ['B', { x: 400, y: 100 }],
    ])
    const sizes = new Map([['A', { width: 200, height: 80 }]])

    const bbox = computeBoundingBox(positions, sizes)

    // Only A contributes: left=0, top=60, right=200, bottom=140
    expect(bbox.minX).toBe(0)
    expect(bbox.minY).toBe(60)
    expect(bbox.maxX).toBe(200)
    expect(bbox.maxY).toBe(140)
  })
})

describe('computeCanvasSize', () => {
  it('returns width = maxX-minX + 2*padding, height = maxY-minY + 2*padding', () => {
    const bbox = { minX: 0, minY: 50, maxX: 500, maxY: 250 }
    const result = computeCanvasSize(bbox, 40)

    // width = 500-0+80 = 580, but min 800
    // height = 250-50+80 = 280, but min 600
    expect(result.width).toBe(800)
    expect(result.height).toBe(600)
  })

  it('uses computed size when larger than minimum', () => {
    const bbox = { minX: 0, minY: 0, maxX: 1200, maxY: 900 }
    const result = computeCanvasSize(bbox, 50)

    expect(result.width).toBe(1200 - 0 + 100) // 1300
    expect(result.height).toBe(900 - 0 + 100) // 1000
  })

  it('enforces minimum width of 800 and height of 600', () => {
    const bbox = { minX: 100, minY: 100, maxX: 200, maxY: 200 }
    const result = computeCanvasSize(bbox, 20)

    expect(result.width).toBe(800)
    expect(result.height).toBe(600)
  })
})

describe('fitToScreen', () => {
  it('zooms in (zoom >= 1) when content is smaller than viewport', () => {
    // Content 100x100 fits easily in 800x600 viewport → zoom = min(8, 6) = 6, capped to 3
    const result = fitToScreen({ width: 100, height: 100 }, 800, 600)

    expect(result.zoom).toBe(3)
    expect(Number.isFinite(result.scrollX)).toBe(true)
    expect(Number.isFinite(result.scrollY)).toBe(true)
  })

  it('zooms out (zoom < 1) when content is larger than viewport', () => {
    // Content 2000x1500 is larger than 800x600 viewport → zoom = min(0.4, 0.4) = 0.4
    const result = fitToScreen({ width: 2000, height: 1500 }, 800, 600)

    expect(result.zoom).toBeCloseTo(0.4, 5)
    expect(Number.isFinite(result.scrollX)).toBe(true)
    expect(Number.isFinite(result.scrollY)).toBe(true)
  })

  it('centers content in viewport', () => {
    const bbox = { width: 400, height: 300 }
    const vw = 1000
    const vh = 800

    const result = fitToScreen(bbox, vw, vh)

    // zoom = min(1000/400, 800/300) = min(2.5, 2.667) = 2.5
    expect(result.zoom).toBe(2.5)
    // scrollX = (1000 - 400*2.5)/2 = 0
    expect(result.scrollX).toBe(0)
    // scrollY = (800 - 300*2.5)/2 = (800-750)/2 = 25
    expect(result.scrollY).toBe(25)
  })

  it('caps zoom between 0.1 and 3', () => {
    // Tiny content in huge viewport — would be zoom 100, capped to 3
    const r1 = fitToScreen({ width: 10, height: 10 }, 10000, 10000)
    expect(r1.zoom).toBe(3)

    // Huge content in tiny viewport — would be zoom 0.01, capped to 0.1
    const r2 = fitToScreen({ width: 100000, height: 100000 }, 100, 100)
    expect(r2.zoom).toBe(0.1)
  })
})
