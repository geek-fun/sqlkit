<script setup lang="ts">
import type { ComboboxOption } from './types'
import { computed, nextTick, ref, useId, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Spinner } from '@/components/ui/spinner'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<{
    options: ComboboxOption[]
    modelValue: string
    placeholder?: string
    searchPlaceholder?: string
    disabled?: boolean
    loading?: boolean
    emptyText?: string
    allowCreate?: boolean
    createText?: string
    searchThreshold?: number
    variant?: 'outline' | 'ghost'
    class?: string
  }>(),
  {
    placeholder: 'Select...',
    searchPlaceholder: 'Search...',
    disabled: false,
    loading: false,
    emptyText: 'No results',
    allowCreate: false,
    createText: 'Create',
    searchThreshold: 10,
    variant: 'outline',
    class: undefined,
  },
)

const emits = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'open', isOpen: boolean): void
}>()

const open = ref(false)
const searchQuery = ref('')
const searchInputRef = ref<HTMLInputElement>()
const highlightedIndex = ref(-1)
const listRef = ref<HTMLDivElement>()
const listboxId = useId()

const showSearch = computed(
  () => props.options.length > props.searchThreshold || props.allowCreate,
)

const selectedLabel = computed(() => {
  const found = props.options.find(opt => opt.value === props.modelValue)
  if (found)
    return found.label
  if (props.allowCreate && props.modelValue)
    return props.modelValue
  return undefined
})

const filteredOptions = computed(() => {
  if (!searchQuery.value.trim()) {
    return props.options
  }
  const query = searchQuery.value.toLowerCase()
  return props.options.filter(
    opt => opt.label.toLowerCase().includes(query) || opt.value.toLowerCase().includes(query),
  )
})

const showCreateNew = computed(() => {
  const trimmed = searchQuery.value.trim()
  if (!props.allowCreate || !trimmed)
    return false
  return !props.options.some(opt => opt.value === trimmed || opt.label === trimmed)
})

const navigableItems = computed(() => {
  const items: Array<{ type: 'option', value: string } | { type: 'create', value: string }>
    = filteredOptions.value
      .filter((opt: ComboboxOption) => !opt.disabled)
      .map((opt: ComboboxOption) => ({ type: 'option' as const, value: opt.value }))
  if (showCreateNew.value)
    items.push({ type: 'create', value: searchQuery.value.trim() })
  return items
})

const activeDescendantId = computed(() => {
  if (!open.value || highlightedIndex.value < 0)
    return undefined
  const item = navigableItems.value[highlightedIndex.value]
  if (!item)
    return undefined
  return item.type === 'create' ? `${listboxId}-create` : `${listboxId}-opt-${item.value}`
})

function selectOption(value: string) {
  emits('update:modelValue', value)
  open.value = false
}

function findOptionIndex(optionValue: string) {
  return navigableItems.value.findIndex(
    (item: { type: 'option' | 'create', value: string }) => item.type === 'option' && item.value === optionValue,
  )
}

function findCreateIndex() {
  return navigableItems.value.findIndex((i: { type: 'option' | 'create', value: string }) => i.type === 'create')
}

async function scrollHighlightedIntoView() {
  await nextTick()
  const list = listRef.value
  if (!list)
    return
  const highlighted = list.querySelector<HTMLElement>('[data-highlighted]')
  highlighted?.scrollIntoView({ block: 'nearest' })
}

async function handleKeydown(e: KeyboardEvent) {
  if (!open.value) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      open.value = true
    }
    return
  }

  const count = navigableItems.value.length

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    highlightedIndex.value = count === 0 ? -1 : (highlightedIndex.value + 1) % count
    scrollHighlightedIntoView()
  }
  else if (e.key === 'ArrowUp') {
    e.preventDefault()
    highlightedIndex.value = count === 0 ? -1 : (highlightedIndex.value - 1 + count) % count
    scrollHighlightedIntoView()
  }
  else if (e.key === 'Enter') {
    e.preventDefault()
    if (highlightedIndex.value >= 0 && highlightedIndex.value < count) {
      selectOption(navigableItems.value[highlightedIndex.value].value)
    }
  }
  else if (e.key === 'Escape') {
    open.value = false
  }
  else if (e.key === 'Tab') {
    open.value = false
  }
}

watch(open, async (isOpen: boolean) => {
  if (!isOpen) {
    searchQuery.value = ''
    highlightedIndex.value = -1
  }
  else if (showSearch.value) {
    await nextTick()
    searchInputRef.value?.focus()
  }
  emits('open', isOpen)
})

watch(searchQuery, () => {
  highlightedIndex.value = -1
})
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <Button
        :variant="variant"
        role="combobox"
        :aria-expanded="open"
        aria-haspopup="listbox"
        :aria-controls="listboxId"
        :aria-activedescendant="activeDescendantId"
        :disabled="disabled"
        :class="
          cn(
            'w-full min-w-0 overflow-hidden justify-between font-normal focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
            props.class,
          )
        "
        @keydown="handleKeydown"
      >
        <slot name="selected-prepend" />
        <template v-if="open && showSearch">
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            :placeholder="searchPlaceholder || placeholder"
            class="text-sm text-foreground outline-none border-0 bg-transparent flex-1 min-w-0 placeholder:text-muted-foreground"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck="false"
            @click.stop
          >
          <svg class="ml-2 opacity-50 shrink-0 h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
        </template>
        <template v-else>
          <span v-if="selectedLabel" class="truncate">{{ selectedLabel }}</span>
          <span v-else class="text-muted-foreground truncate">{{ placeholder }}</span>
          <svg class="ml-2 opacity-50 shrink-0 h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m6 9 6 6 6-6" />
          </svg>
        </template>
      </Button>
    </PopoverTrigger>

    <PopoverContent
      align="start"
      class="p-0 w-[unset]"
      style="width: var(--radix-popover-trigger-width); min-width: var(--radix-popover-trigger-width)"
    >
      <div :id="listboxId" ref="listRef" role="listbox" class="py-1 pl-1 pr-0 max-h-[280px] overflow-y-auto">
        <div
          v-if="loading"
          class="text-sm text-muted-foreground py-4 flex gap-2 items-center justify-center"
        >
          <Spinner class="h-4 w-4" />
          Loading...
        </div>

        <template v-else>
          <template v-for="(option, index) in filteredOptions" :key="option.value">
            <!-- Group header -->
            <div
              v-if="option.group && (index === 0 || filteredOptions[index - 1]?.group !== option.group)"
              class="text-xs text-muted-foreground font-medium px-2 py-1 pt-2 select-none"
            >
              {{ option.group }}
            </div>
            <div
              :id="`${listboxId}-opt-${option.value}`"
              role="option"
              :aria-selected="option.value === modelValue"
              :aria-disabled="option.disabled || undefined"
              :data-highlighted="
                navigableItems[highlightedIndex]?.type === 'option'
                  && navigableItems[highlightedIndex]?.value === option.value
                  ? ''
                  : undefined
              "
              class="text-sm px-1 py-0.5 outline-none flex cursor-pointer select-none transition-colors items-center relative hover:text-accent-foreground hover:bg-accent" :class="[
                option.disabled && 'pointer-events-none opacity-50',
                option.value === modelValue && 'bg-accent text-accent-foreground',
                navigableItems[highlightedIndex]?.type === 'option'
                  && navigableItems[highlightedIndex]?.value === option.value
                  && 'bg-accent text-accent-foreground',
              ]"
              @click="!option.disabled && selectOption(option.value)"
              @mouseenter="highlightedIndex = findOptionIndex(option.value)"
            >
              <slot name="option" :option="option">
                {{ option.label }}
              </slot>
            </div>
          </template>

          <div
            v-if="filteredOptions.length === 0 && searchQuery && !showCreateNew"
            class="text-sm text-muted-foreground px-2 py-8 text-center flex flex-col gap-2 items-center"
          >
            <svg class="opacity-40 h-5 w-5" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.3-4.3" />
            </svg>
            <span>No results for "{{ searchQuery }}"</span>
          </div>

          <div
            v-if="filteredOptions.length === 0 && !searchQuery"
            class="text-sm text-muted-foreground px-2 py-8 text-center flex flex-col gap-2 items-center"
          >
            <svg class="opacity-40 h-5 w-5" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 9v4" />
              <path d="M12 17h.01" />
              <path d="M3.6 9h16.8" />
              <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z" />
            </svg>
            {{ emptyText }}
          </div>

          <div
            v-if="showCreateNew"
            :id="`${listboxId}-create`"
            role="option"
            :data-highlighted="navigableItems[highlightedIndex]?.type === 'create' ? '' : undefined"
            class="text-sm text-green-600 px-1.5 py-1 outline-none flex gap-2 cursor-pointer select-none italic transition-colors items-center relative hover:bg-accent" :class="[
              navigableItems[highlightedIndex]?.type === 'create' && 'bg-accent',
            ]"
            @click="selectOption(searchQuery.trim())"
            @mouseenter="highlightedIndex = findCreateIndex()"
          >
            <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 5v14" />
              <path d="M5 12h14" />
            </svg>
            {{ createText }}: "{{ searchQuery.trim() }}"
          </div>
        </template>
      </div>
    </PopoverContent>
  </Popover>
</template>
