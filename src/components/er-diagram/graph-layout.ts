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
): Record<string, { x: number, y: number }> {
  const g = new dagre.graphlib.Graph()
  g.setGraph({
    rankdir: 'TB',
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

  const positions: Record<string, { x: number, y: number }> = {}
  g.nodes().forEach((nodeName: string) => {
    const node = g.node(nodeName) as { x: number, y: number }
    positions[nodeName] = { x: node.x, y: node.y }
  })

  return positions
}

export function computeCanvasSize(
  positions: Record<string, { x: number, y: number }>,
  nodeWidth: number,
  getHeight: (name: string) => number,
  padding: number,
): { width: number, height: number } {
  let width = 960
  let height = 540
  for (const [name, pos] of Object.entries(positions)) {
    const h = getHeight(name)
    width = Math.max(width, pos.x + nodeWidth / 2 + padding)
    height = Math.max(height, pos.y + h / 2 + padding)
  }
  return { width, height }
}

export function fitToScreen(
  canvasSize: { width: number, height: number },
  viewportWidth: number,
  viewportHeight: number,
): { zoom: number, scrollX: number, scrollY: number } {
  const rawZoom = Math.min(
    viewportWidth / canvasSize.width,
    viewportHeight / canvasSize.height,
  )
  const zoom = Math.max(0.1, Math.min(3, rawZoom))
  const scrollX = (viewportWidth - canvasSize.width * zoom) / 2
  const scrollY = (viewportHeight - canvasSize.height * zoom) / 2
  return { zoom, scrollX, scrollY }
}
