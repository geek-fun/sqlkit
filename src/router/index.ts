import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    redirect: '/connections',
  },
  {
    path: '/connections',
    name: 'connections',
    component: () => import('@/pages/ConnectionsPage.vue'),
  },
  {
    path: '/transfer',
    name: 'transfer',
    component: () => import('@/pages/TransferPage.vue'),
  },
  {
    path: '/data-studio',
    name: 'data-studio',
    component: () => import('@/pages/DataStudioPage.vue'),
  },
  {
    path: '/queries',
    name: 'queries',
    component: () => import('@/pages/QueriesPage.vue'),
  },
  {
    path: '/history',
    name: 'history',
    component: () => import('@/pages/HistoryPage.vue'),
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('@/pages/SettingsPage.vue'),
  },
]

export const router = createRouter({
  history: createWebHistory(),
  routes,
})
