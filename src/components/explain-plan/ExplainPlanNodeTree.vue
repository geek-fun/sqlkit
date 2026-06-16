<script setup lang="ts">
import type { ExplainPlanNode } from '@/types/explainPlan'
import { computed, ref } from 'vue'
import { getCostColor } from '@/utils/explainPlanParser'

defineOptions({
  name: 'ExplainPlanNodeTree',
})

const props = defineProps<{
  node: ExplainPlanNode
  totalCost?: number
}>()

const collapsed = ref(false)

function toggle() {
  if (props.node.children.length > 0)
    collapsed.value = !collapsed.value
}

const hasActual = !!props.node.actualRows
const rowDiffers = hasActual && props.node.rows && props.node.actualRows !== props.node.rows
const rowRatio = rowDiffers && props.node.rows && props.node.rows > 0
  ? Math.round((props.node.actualRows! / props.node.rows) * 100)
  : null

const costColorClass = computed(() => {
  const cost = props.node.totalCost || 0
  const total = props.totalCost || 0
  const color = getCostColor(cost, total)
  return {
    green: 'bg-green-500',
    yellow: 'bg-yellow-500',
    red: 'bg-red-500',
  }[color]
})
</script>

<template>
  <div>
    <div
      class="text-xs px-2 py-1 border rounded bg-background flex gap-1.5 cursor-pointer items-center hover:bg-muted/30"
      :class="{ 'border-green-300 dark:border-green-700': hasActual }"
      @click="toggle"
    >
      <span class="rounded-full shrink-0 h-3 w-2" :class="costColorClass" />
      <svg
        v-if="node.children.length > 0 && collapsed"
        class="text-muted-foreground shrink-0 h-3 w-3"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="m9 18 6-6-6-6" />
      </svg>
      <svg
        v-else-if="node.children.length > 0"
        class="text-muted-foreground shrink-0 h-3 w-3"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
      <span
        v-else
        class="shrink-0 h-3 w-3"
      />
      <span class="font-medium px-1 py-0.5 rounded bg-muted shrink-0">{{ node.nodeType }}</span>
      <span
        v-if="node.relation"
        class="text-blue-600 shrink-0 max-w-[120px] truncate dark:text-blue-400"
      >{{ node.relation }}</span>
      <span
        v-if="node.index"
        class="text-emerald-600 shrink-0 dark:text-emerald-400"
      >[{{ node.index }}]</span>
      <span
        v-if="node.cost"
        class="text-muted-foreground shrink-0 tabular-nums"
      >c:{{ node.cost }}</span>
      <span
        v-if="node.rows"
        class="text-amber-600 shrink-0 tabular-nums dark:text-amber-400"
      >e:{{ node.rows }}</span>
      <span
        v-if="hasActual"
        class="font-semibold shrink-0 tabular-nums"
        :class="rowDiffers ? 'text-green-600 dark:text-green-400' : 'text-muted-foreground'"
      >
        a:{{ node.actualRows }}<span v-if="rowDiffers && rowRatio">({{ rowRatio }}%)</span>
      </span>
      <span
        v-if="node.details.length"
        class="text-muted-foreground/40 ml-auto shrink-0 whitespace-nowrap text-ellipsis overflow-hidden"
        :title="node.details.join('\n')"
      >{{ node.details.join(' ') }}</span>
    </div>
    <div
      v-if="node.children.length && !collapsed"
      class="ml-5 mt-px pl-3 border-l space-y-px"
    >
      <ExplainPlanNodeTree
        v-for="child in node.children"
        :key="child.id"
        :node="child"
        :total-cost="totalCost"
      />
    </div>
  </div>
</template>
