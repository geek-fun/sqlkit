<script setup lang="ts">
import type { ExplainPlanNode, ExplainResult } from '@/types/explainPlan'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Spinner } from '@/components/ui/spinner'
import ExplainPlanNodeTree from './ExplainPlanNodeTree.vue'
import ExplainPlanRawView from './ExplainPlanRawView.vue'
import ExplainPlanSummaryTable from './ExplainPlanSummaryTable.vue'

const props = defineProps<{
  explainResult: ExplainResult | null
  loading: boolean
  error: string | null
}>()

const { t } = useI18n()
const activeView = ref<'tree' | 'summary' | 'raw'>('tree')

// Use stored values from ExplainResult (computed by parseExplainResult)
const totalCost = computed(() => props.explainResult?.totalCost)
const mostExpensive = computed(() => props.explainResult?.mostExpensiveNode)

const nodeCount = computed(() => {
  if (!props.explainResult?.nodes.length)
    return 0
  function count(nodes: ExplainPlanNode[]): number {
    let c = nodes.length
    for (const n of nodes) {
      c += count(n.children || [])
    }
    return c
  }
  return count(props.explainResult.nodes)
})

const formattedTotalCost = computed(() => {
  const c = totalCost.value
  if (c === undefined)
    return undefined
  return Number.isInteger(c) ? c.toFixed(0) : c.toFixed(2)
})
</script>

<template>
  <div class="bg-background flex flex-col h-full min-h-0">
    <!-- Summary bar -->
    <div
      v-if="explainResult && !loading"
      class="text-xs px-3 py-1.5 border-b flex shrink-0 gap-3 items-center"
    >
      <span class="font-medium px-2 py-0.5 border rounded bg-muted inline-flex gap-1 items-center">
        {{ t('pages.queries.explain.title') }}
      </span>
      <span class="text-muted-foreground">
        {{ explainResult.databaseType.toUpperCase() }} &middot; {{ t('pages.queries.explain.nodeCount', { count: nodeCount }) }}
      </span>
      <span class="flex-1" />
      <span v-if="formattedTotalCost !== undefined" class="tabular-nums">
        {{ t('pages.queries.explain.summary.totalCost', { cost: formattedTotalCost }) }}
      </span>
      <span v-if="mostExpensive" class="text-muted-foreground">
        | {{ t('pages.queries.explain.summary.mostExpensive', { node: mostExpensive }) }}
      </span>
      <span v-else-if="formattedTotalCost === undefined" class="text-muted-foreground">
        {{ t('pages.queries.explain.summary.noCostData') }}
      </span>
    </div>

    <!-- View tabs -->
    <div
      v-if="explainResult && !loading"
      class="px-3 py-1 border-b flex shrink-0 gap-1"
    >
      <button
        class="text-xs font-medium px-2 py-0.5 rounded transition-colors"
        :class="activeView === 'tree' ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'"
        @click="activeView = 'tree'"
      >
        {{ t('pages.queries.explain.treeView') }}
      </button>
      <button
        class="text-xs font-medium px-2 py-0.5 rounded transition-colors"
        :class="activeView === 'summary' ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'"
        @click="activeView = 'summary'"
      >
        {{ t('pages.queries.explain.summaryView') }}
      </button>
      <button
        class="text-xs font-medium px-2 py-0.5 rounded transition-colors"
        :class="activeView === 'raw' ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'"
        @click="activeView = 'raw'"
      >
        {{ t('pages.queries.explain.rawView') }}
      </button>
    </div>

    <!-- Loading state -->
    <div
      v-if="loading"
      class="flex flex-1 items-center justify-center"
    >
      <div class="text-center">
        <Spinner class="mx-auto mb-2" />
        <p class="text-sm text-muted-foreground">
          {{ t('pages.queries.explain.running') }}
        </p>
      </div>
    </div>

    <!-- Error state -->
    <div
      v-else-if="error"
      class="p-4 flex flex-1 items-start justify-center"
    >
      <div class="text-sm text-destructive px-3 py-2 border border-destructive/30 rounded bg-destructive/5 max-w-xl">
        <p>{{ error }}</p>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="!explainResult"
      class="flex flex-1 items-center justify-center"
    >
      <p class="text-sm text-muted-foreground">
        {{ t('pages.queries.explain.empty') }}
      </p>
    </div>

    <!-- Content views -->
    <div
      v-else
      class="flex-1 min-h-0 overflow-auto"
    >
      <div
        v-if="activeView === 'tree'"
        class="mx-auto p-2 max-w-5xl space-y-px"
      >
        <ExplainPlanNodeTree
          v-for="node in explainResult.nodes"
          :key="node.id"
          :node="node"
          :total-cost="totalCost"
        />
      </div>
      <ExplainPlanSummaryTable
        v-else-if="activeView === 'summary' && explainResult"
        :nodes="explainResult.nodes"
      />
      <ExplainPlanRawView
        v-else-if="activeView === 'raw' && explainResult"
        :raw="explainResult.raw"
        :format="explainResult.format"
      />
    </div>
  </div>
</template>
