/**
 * @jest-environment jsdom
 * @jest-environment-options {"customExportConditions": ["node", "node-addons"]}
 */
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { jdbcApi } from '@/datasources'
import { enUS } from '@/lang/enUS'
import JreDriverSettings from '@/views/setting/jre-driver-section.vue'

let receive: (event: { payload: unknown }) => void
jest.mock('@tauri-apps/api/event', () => ({ listen: jest.fn((_name, callback) => {
  receive = callback
  return Promise.resolve(jest.fn())
}) }))
jest.mock('@/assets/images/java-logo.svg', () => 'java-logo.svg')
jest.mock('@/composables', () => jest.requireActual('@/composables/useDownloadEvents'))
jest.mock('@/composables/useDatabaseIcon', () => ({ useDatabaseIcon: () => ({ getDatabaseIcon: () => '' }) }))
jest.mock('@/store', () => ({ dbTypeFromBackend: {} }))
jest.mock('@/components/ui/card', () => ({ Card: { template: '<div><slot /></div>' }, CardContent: { template: '<div><slot /></div>' }, CardHeader: { template: '<div><slot /></div>' }, CardTitle: { template: '<h2><slot /></h2>' }, CardDescription: { template: '<p><slot /></p>' } }))
jest.mock('@/components/ui/badge', () => ({ Badge: { template: '<span><slot /></span>' } }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/datasources', () => ({ jdbcApi: {
  checkJreStatus: jest.fn(),
  checkBridgeStatus: jest.fn(),
  listDrivers: jest.fn(),
  downloadDriver: jest.fn(),
  checkJreUpdate: jest.fn(),
} }))
const api = jdbcApi as jest.Mocked<typeof jdbcApi>
const driver = { db_type: 'h2', name: 'H2 Database', installed: false, filename: null, file_size: null, version_cap: null, resolved_version: null }
const render = () => mount(JreDriverSettings, { global: { plugins: [createI18n({ legacy: false, locale: 'enUS', messages: { enUS } })] } })

beforeEach(() => {
  jest.clearAllMocks()
  api.checkJreStatus.mockResolvedValue({ source: 'managed', version: '25', installed: true, path: '/example/java' })
  api.checkBridgeStatus.mockResolvedValue({ installed: true, current_version: '0.8.8', path: '/example/bridge.jar' })
  api.listDrivers.mockResolvedValue([{ ...driver }])
})

it('shows preparation, real byte progress, verification and completion at the affected driver', async () => {
  let finish!: () => void
  api.downloadDriver.mockImplementation(() => new Promise((resolve) => {
    finish = resolve
  }))
  const wrapper = render()
  await flushPromises()
  await wrapper.get('button[aria-label="H2 Database: Download"]').trigger('click')
  expect(wrapper.text()).toContain('Preparing download')
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined()
  await flushPromises()
  receive({ payload: { phase: 'progress', id: 'h2', kind: 'driver', downloaded: 50, total: 100 } })
  await flushPromises()
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('50')
  receive({ payload: { phase: 'complete', id: 'h2', kind: 'driver' } })
  await flushPromises()
  expect(wrapper.text()).toContain('Installing and verifying')
  api.listDrivers.mockResolvedValue([{ ...driver, installed: true }])
  finish()
  await flushPromises()
  expect(wrapper.text()).toContain('Installation complete')
  expect(wrapper.find('[role="progressbar"]').exists()).toBe(false)
  wrapper.unmount()
})

it('load errors remain visible with a working retry instead of reporting not installed', async () => {
  api.checkJreStatus.mockRejectedValueOnce(new Error('Runtime lookup unavailable'))
  const wrapper = render()
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('Runtime lookup unavailable')
  const retry = wrapper.findAll('button').find(button => button.text() === 'Retry')!
  await retry.trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('/example/java')
  expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  wrapper.unmount()
})

it('keeps an active download visible after returning to settings and reloads its result', async () => {
  let finish!: () => void
  api.downloadDriver.mockImplementation(() => new Promise((resolve) => {
    finish = resolve
  }))
  const first = render()
  await flushPromises()
  await first.get('button[aria-label="H2 Database: Download"]').trigger('click')
  await flushPromises()
  first.unmount()
  receive({ payload: { phase: 'progress', id: 'h2', kind: 'driver', downloaded: 70, total: 100 } })
  const second = render()
  await flushPromises()
  expect(second.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('70')
  api.listDrivers.mockResolvedValue([{ ...driver, installed: true }])
  finish()
  await flushPromises()
  expect(second.text()).toContain('Installation complete')
  expect(second.find('button[aria-label="H2 Database: Download"]').exists()).toBe(false)
  second.unmount()
})
