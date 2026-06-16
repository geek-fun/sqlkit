<script setup lang="ts">
import type { ExplainPlanNode } from '@/types/explainPlan'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  nodes: ExplainPlanNode[]
}>()

const { t } = useI18n()

type FlatRow = {
  node: ExplainPlanNode
  depth: number
}

function buildFlatRows(nodes: ExplainPlanNode[], depth = 0): FlatRow[] {
  const rows: FlatRow[] = []
  for (const node of nodes) {
    rows.push({ node, depth })
    rows.push(...buildFlatRows(node.children, depth + 1))
  }
  return rows
}

const flatRows = computed(() => buildFlatRows(props.nodes))
</script>

<template>
  <div class="border rounded overflow-auto">
    <table class="text-xs text-left min-w-[700px] w-full">
      <thead class="text-muted-foreground bg-muted/70">
        <tr>
          <th class="font-medium px-2 py-1.5">
            {{ t('pages.queries.explain.columnHeaders.nodeType') }}
          </th>
          <th class="font-medium px-2 py-1.5">
            {{ t('pages.queries.explain.columnHeaders.relation') }}
          </th>
          <th class="font-medium px-2 py-1.5">
            {{ t('pages.queries.explain.columnHeaders.index') }}
          </th>
          <th class="font-medium px-2 py-1.5">
            {{ t('pages.queries.explain.columnHeaders.cost') }}
          </th>
          <th class="font-medium px-2 py-1.5">
            {{ t('pages.queries.explain.columnHeaders.rows') }}
          </th>
          <th class="font-medium px-2 py-1.5">
            {{ t('pages.queries.explain.columnHeaders.details') }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="row in flatRows"
          :key="row.node.id"
          class="border-t"
        >
          <td
            class="font-medium px-2 py-1.5"
            :style="{ paddingLeft: `${8 + row.depth * 20}px` }"
          >
            {{ row.node.title }}
          </td>
          <td class="text-muted-foreground px-2 py-1.5">
            {{ row.node.relation || '-' }}
          </td>
          <td class="text-muted-foreground px-2 py-1.5">
            {{ row.node.index || '-' }}
          </td>
          <td class="px-2 py-1.5 tabular-nums">
            {{ row.node.cost || '-' }}
          </td>
          <td class="px-2 py-1.5 tabular-nums">
            {{ row.node.rows || '-' }}
          </td>
          <td class="text-muted-foreground px-2 py-1.5">
            {{ row.node.details.join('; ') || '-' }}
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
