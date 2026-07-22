import * as dagre from 'dagre'

export type DagreTable = {
  name: string
  width: number
  height: number
}

export type DagreRel = {
  sourceTable: string
  targetTable: string
}

export function computeDagreLayout(
  tables: DagreTable[],
  relationships: DagreRel[],
  direction: 'TB' | 'LR',
): Map<string, { x: number, y: number }> {
  const g = new dagre.graphlib.Graph()
  g.setGraph({
    rankdir: direction,
    nodesep: 80,
    ranksep: 120,
    marginx: 40,
    marginy: 40,
  })
  g.setDefaultEdgeLabel(() => ({}))

  for (const table of tables)
    g.setNode(table.name, { label: table.name, width: table.width, height: table.height })

  for (const rel of relationships)
    g.setEdge(rel.sourceTable, rel.targetTable, { label: '' })

  dagre.layout(g)

  const positions = new Map<string, { x: number, y: number }>()
  g.nodes().forEach((nodeName: string) => {
    const node = g.node(nodeName) as { x: number, y: number }
    positions.set(nodeName, { x: node.x, y: node.y })
  })

  return positions
}

export function computeBoundingBox(
  positions: Map<string, { x: number, y: number }>,
  sizes: Map<string, { width: number, height: number }>,
): { minX: number, minY: number, maxX: number, maxY: number } {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity

  for (const [name, pos] of positions) {
    const size = sizes.get(name)
    if (!size)
      continue

    const left = pos.x - size.width / 2
    const right = pos.x + size.width / 2
    const top = pos.y - size.height / 2
    const bottom = pos.y + size.height / 2

    if (left < minX)
      minX = left
    if (top < minY)
      minY = top
    if (right > maxX)
      maxX = right
    if (bottom > maxY)
      maxY = bottom
  }

  if (minX === Infinity)
    return { minX: 0, minY: 0, maxX: 0, maxY: 0 }

  return { minX, minY, maxX, maxY }
}

export function computeCanvasSize(
  bbox: { minX: number, minY: number, maxX: number, maxY: number },
  padding: number,
): { width: number, height: number } {
  return {
    width: Math.max(800, bbox.maxX - bbox.minX + 2 * padding),
    height: Math.max(600, bbox.maxY - bbox.minY + 2 * padding),
  }
}

export function fitToScreen(
  bbox: { width: number, height: number },
  viewportWidth: number,
  viewportHeight: number,
): { zoom: number, scrollX: number, scrollY: number } {
  const rawZoom = Math.min(
    viewportWidth / bbox.width,
    viewportHeight / bbox.height,
  )
  const zoom = Math.max(0.1, Math.min(3, rawZoom))
  const scrollX = (viewportWidth - bbox.width * zoom) / 2
  const scrollY = (viewportHeight - bbox.height * zoom) / 2
  return { zoom, scrollX, scrollY }
}
