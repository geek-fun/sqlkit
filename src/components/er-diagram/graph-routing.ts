import type { TableRect } from './types'
import { NODE_WIDTH, ROUTE_BLOCK_MARGIN, ROUTE_PADDING } from './types'

export function rangesOverlap(a1: number, a2: number, b1: number, b2: number): boolean {
  return Math.max(a1, b1) <= Math.min(a2, b2)
}

export function routeSideX(rect: TableRect, routeX: number, offset = 0): number {
  if (routeX < rect.x)
    return rect.x - offset
  return rect.x + rect.width + offset
}

export function isVerticalRouteBlocked(
  routeX: number,
  y1: number,
  y2: number,
  ignored: Set<string>,
  rects: TableRect[],
): boolean {
  const top = Math.min(y1, y2)
  const bottom = Math.max(y1, y2)
  return rects.some(
    r =>
      !ignored.has(r.name)
      && routeX >= r.x - ROUTE_BLOCK_MARGIN
      && routeX <= r.x + r.width + ROUTE_BLOCK_MARGIN
      && rangesOverlap(top, bottom, r.y - ROUTE_BLOCK_MARGIN, r.y + r.height + ROUTE_BLOCK_MARGIN),
  )
}

export function isHorizontalRouteBlocked(
  y: number,
  x1: number,
  x2: number,
  ignored: Set<string>,
  rects: TableRect[],
): boolean {
  const left = Math.min(x1, x2)
  const right = Math.max(x1, x2)
  return rects.some(
    r =>
      !ignored.has(r.name)
      && y >= r.y - ROUTE_BLOCK_MARGIN
      && y <= r.y + r.height + ROUTE_BLOCK_MARGIN
      && rangesOverlap(left, right, r.x - ROUTE_BLOCK_MARGIN, r.x + r.width + ROUTE_BLOCK_MARGIN),
  )
}

export function candidateRouteXs(source: TableRect, target: TableRect, rects: TableRect[]): number[] {
  const candidates = new Set<number>()
  const minLeft = Math.min(source.x, target.x)
  const maxRight = Math.max(source.x + source.width, target.x + target.width)

  candidates.add(minLeft - ROUTE_PADDING)
  candidates.add(maxRight + ROUTE_PADDING)

  if (source.x + source.width + ROUTE_PADDING <= target.x)
    candidates.add((source.x + source.width + target.x) / 2)
  if (target.x + target.width + ROUTE_PADDING <= source.x)
    candidates.add((target.x + target.width + source.x) / 2)

  const columns = [...new Set(rects.map(r => r.x))].sort((a, b) => a - b)
  for (let i = 0; i < columns.length - 1; i++) {
    const gap = columns[i + 1] - (columns[i] + NODE_WIDTH)
    if (gap >= ROUTE_PADDING)
      candidates.add((columns[i] + NODE_WIDTH + columns[i + 1]) / 2)
  }

  return [...candidates].sort((a, b) => {
    const sa = routeSideX(source, a)
    const ta = routeSideX(target, a)
    const sb = routeSideX(source, b)
    const tb = routeSideX(target, b)
    return Math.abs(a - sa) + Math.abs(a - ta) - (Math.abs(b - sb) + Math.abs(b - tb))
  })
}

export function computeRelationshipPath(
  sourceTable: string,
  targetTable: string,
  rects: Record<string, TableRect>,
): string {
  const source = rects[sourceTable]
  const target = rects[targetTable]
  if (!source || !target)
    return ''

  // Simple center-to-center cubic bezier — no orthogonal routing edge cases
  const x1 = source.x + source.width / 2
  const y1 = source.y + source.height / 2
  const x2 = target.x + target.width / 2
  const y2 = target.y + target.height / 2
  const dx = Math.abs(x2 - x1) * 0.4
  return `M ${x1} ${y1} C ${x1} ${y1 + dx}, ${x2} ${y2 - dx}, ${x2} ${y2}`
}

export function buildTableRectMap(
  tableNames: string[],
  nodePositions: Record<string, { x: number, y: number }>,
  getHeight: (name: string) => number,
): Record<string, TableRect> {
  const map: Record<string, TableRect> = {}
  for (const name of tableNames) {
    const pos = nodePositions[name]
    if (!pos)
      continue
    const h = getHeight(name)
    map[name] = {
      name,
      x: pos.x - NODE_WIDTH / 2,
      y: pos.y - h / 2,
      width: NODE_WIDTH,
      height: h,
    }
  }
  return map
}
