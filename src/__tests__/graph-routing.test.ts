import type { TableRect } from '@/components/er-diagram/types'
/**
 * @jest-environment node
 */
import {
  buildTableRectMap,
  candidateRouteXs,
  computeRelationshipPath,
  rangesOverlap,
  routeSideX,
} from '@/components/er-diagram/graph-routing'

const rectA: TableRect = { name: 'A', x: 0, y: 0, width: 220, height: 100 }
const rectB: TableRect = { name: 'B', x: 300, y: 0, width: 220, height: 100 }
const rectBlocker: TableRect = { name: 'Blocker', x: 150, y: 50, width: 220, height: 100 }

describe('rangesOverlap', () => {
  it('returns true when ranges overlap', () => {
    expect(rangesOverlap(0, 10, 5, 15)).toBe(true)
  })

  it('returns true when one range is inside another', () => {
    expect(rangesOverlap(0, 20, 5, 10)).toBe(true)
  })

  it('returns true when ranges touch at a point', () => {
    expect(rangesOverlap(0, 10, 10, 20)).toBe(true)
  })

  it('returns false when ranges do not overlap', () => {
    expect(rangesOverlap(0, 10, 20, 30)).toBe(false)
  })

  it('returns false when ranges are adjacent but not touching', () => {
    expect(rangesOverlap(0, 10, 11, 20)).toBe(false)
  })
})

describe('routeSideX', () => {
  it('returns rect.x - offset when routeX is left of rect', () => {
    expect(routeSideX(rectA, -50)).toBe(0)
  })

  it('returns rect.x when routeX is left of rect with default offset 0', () => {
    expect(routeSideX(rectA, -50)).toBe(0)
  })

  it('returns rect.x + width + offset when routeX is right of rect', () => {
    expect(routeSideX(rectA, 300)).toBe(220)
  })

  it('uses provided offset when routeX is left of rect', () => {
    expect(routeSideX(rectA, -50, 5)).toBe(-5)
  })

  it('uses provided offset when routeX is right of rect', () => {
    expect(routeSideX(rectA, 300, 10)).toBe(230)
  })
})

describe('candidateRouteXs', () => {
  it('returns sorted candidates by proximity to source and target', () => {
    const candidates = candidateRouteXs(rectA, rectB, [rectA, rectB])
    expect(candidates.length).toBeGreaterThan(0)
    // First candidate should be the best (closest to both rects)
    expect(candidates[0]).toBeGreaterThan(rectA.x)
    expect(candidates[0]).toBeLessThan(rectB.x + rectB.width)
  })

  it('includes the midpoint gap when rects are side by side', () => {
    const candidates = candidateRouteXs(rectA, rectB, [rectA, rectB])
    // rectA ends at 220, rectB starts at 300, gap is 80 >= 56 (ROUTE_PADDING)
    // So midpoint should be (220 + 300) / 2 = 260
    expect(candidates).toContain(260)
  })

  it('includes left and right edge padding candidates', () => {
    const candidates = candidateRouteXs(rectA, rectB, [rectA, rectB])
    // minLeft is 0, so 0 - 56 = -56
    expect(candidates).toContain(-56)
    // maxRight is 300 + 220 = 520, so 520 + 56 = 576
    expect(candidates).toContain(576)
  })
})

describe('computeRelationshipPath', () => {
  it('returns empty string when source table is not in rectMap', () => {
    const rectMap: Record<string, TableRect> = { B: rectB }
    expect(computeRelationshipPath('A', 'B', rectMap)).toBe('')
  })

  it('returns empty string when target table is not in rectMap', () => {
    const rectMap: Record<string, TableRect> = { A: rectA }
    expect(computeRelationshipPath('A', 'B', rectMap)).toBe('')
  })

  it('returns a valid SVG path for two side-by-side rects', () => {
    const rectMap: Record<string, TableRect> = { A: rectA, B: rectB }
    const path = computeRelationshipPath('A', 'B', rectMap)
    expect(path).toMatch(/^M /)
    expect(path).toContain(' L ')
  })

  it('returns a valid path that avoids a blocker rect between source and target', () => {
    const rectMap: Record<string, TableRect> = {
      A: rectA,
      B: rectB,
      Blocker: rectBlocker,
    }
    const path = computeRelationshipPath('A', 'B', rectMap)
    expect(path).toMatch(/^M /)
    expect(path).toContain(' L ')
    // Path should use a routeX that's not through the blocker
    // Verify it has exactly 3 L segments (orthogonal route: right, down, left)
    const segments = path.split(' L ')
    expect(segments.length).toBe(4) // M start + 3 L segments
  })
})

describe('buildTableRectMap', () => {
  it('builds a rect map from table names and positions', () => {
    const nodePositions: Record<string, { x: number, y: number }> = {
      A: { x: 110, y: 50 },
      B: { x: 410, y: 50 },
    }
    const getHeight = (_name: string) => 100
    const rectMap = buildTableRectMap(['A', 'B'], nodePositions, getHeight)
    const keys = Object.keys(rectMap)
    expect(keys).toHaveLength(2)
    expect(keys).toContain('A')
    expect(keys).toContain('B')
    const rectA = rectMap.A
    expect(rectA.x).toBe(0) // 110 - 220/2
    expect(rectA.y).toBe(0) // 50 - 100/2
    expect(rectA.width).toBe(220)
    expect(rectA.height).toBe(100)
  })

  it('skips tables not found in nodePositions', () => {
    const nodePositions: Record<string, { x: number, y: number }> = {
      A: { x: 110, y: 50 },
    }
    const getHeight = (_name: string) => 100
    const rectMap = buildTableRectMap(['A', 'B'], nodePositions, getHeight)
    expect(Object.keys(rectMap)).toHaveLength(1)
  })
})
