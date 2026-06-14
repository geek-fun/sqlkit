<script setup lang="ts">
import { RouterLink, useRoute } from 'vue-router'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

const route = useRoute()

const menuItems = [
  { id: 'connections', label: 'Connections', icon: 'dns', path: '/connections' },
  { id: 'queries', label: 'Queries', icon: 'code', path: '/queries' },
  { id: 'data-studio', label: 'Data Studio', icon: 'smart_toy', path: '/data-studio' },
  { id: 'transfer', label: 'Transfer', icon: 'import_export', path: '/transfer' },
  { id: 'history', label: 'History', icon: 'history', path: '/history' },
]

const isActive = (path: string) => route.path === path
</script>

<template>
  <aside class="border-r bg-background flex flex-shrink-0 flex-col h-full w-16 z-50">
    <!-- Logo -->
    <div class="border-b flex h-14 items-center justify-center">
      <div class="text-lg text-white font-bold rounded-md bg-primary flex h-8 w-8 items-center justify-center">
        S
      </div>
    </div>

    <!-- Navigation -->
    <nav class="px-2 py-4 flex-1 overflow-y-auto space-y-2">
      <TooltipProvider>
        <RouterLink
          v-for="item in menuItems"
          :key="item.id"
          :to="item.path"
          custom
          #="{ navigate }"
        >
          <Tooltip>
            <TooltipTrigger as-child>
              <button
                class="mx-auto rounded-md flex h-10 w-10 cursor-pointer transition-colors items-center justify-center relative" :class="[
                  isActive(item.path)
                    ? 'bg-secondary text-primary border border-border shadow-sm'
                    : 'text-muted-foreground hover:bg-secondary hover:text-foreground',
                ]"
                @click="navigate"
              >
                <!-- DNS icon -->
                <svg
                  v-if="item.icon === 'dns'"
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <ellipse cx="12" cy="5" rx="9" ry="3" />
                  <path d="M3 5v14a9 3 0 0 0 18 0V5" />
                  <path d="M3 12a9 3 0 0 0 18 0" />
                </svg>

                <!-- Import/Export icon -->
                <svg
                  v-if="item.icon === 'import_export'"
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M12 3v18" />
                  <path d="m8 7 4-4 4 4" />
                  <path d="m16 17-4 4-4-4" />
                </svg>

                <!-- Smart Toy / AI icon -->
                <svg
                  v-if="item.icon === 'smart_toy'"
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <rect x="3" y="11" width="18" height="10" rx="2" />
                  <circle cx="12" cy="5" r="2" />
                  <path d="M12 7v4" />
                  <line x1="8" y1="16" x2="8" y2="16" />
                  <line x1="16" y1="16" x2="16" y2="16" />
                </svg>

                <!-- Code icon -->
                <svg
                  v-if="item.icon === 'code'"
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <polyline points="16 18 22 12 16 6" />
                  <polyline points="8 6 2 12 8 18" />
                </svg>

                <!-- History icon -->
                <svg
                  v-if="item.icon === 'history'"
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <circle cx="12" cy="12" r="10" />
                  <polyline points="12 6 12 12 16 14" />
                </svg>

                <!-- AI Badge -->
                <span
                  v-if="false"
                  class="text-[8px] text-white font-bold px-1 rounded-full bg-primary right-1 top-1 absolute"
                >
                  AI
                </span>
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>{{ item.label }}</p>
            </TooltipContent>
          </Tooltip>
        </RouterLink>
      </TooltipProvider>
    </nav>

    <!-- Settings at bottom -->
    <div class="px-2 py-4 border-t">
      <TooltipProvider>
        <RouterLink to="/settings" custom #="{ navigate }">
          <Tooltip>
            <TooltipTrigger as-child>
              <button
                class="mx-auto rounded-md flex h-10 w-10 cursor-pointer transition-colors items-center justify-center" :class="[
                  isActive('/settings')
                    ? 'bg-secondary text-primary border border-border shadow-sm'
                    : 'text-muted-foreground hover:bg-secondary hover:text-foreground',
                ]"
                @click="navigate"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path
                    d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
                  />
                  <circle cx="12" cy="12" r="3" />
                </svg>
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>Settings</p>
            </TooltipContent>
          </Tooltip>
        </RouterLink>
      </TooltipProvider>
    </div>
  </aside>
</template>
