/**
 * @jest-environment jsdom
 * @jest-environment-options {"customExportConditions": ["node", "node-addons"]}
 */
import { getVersion } from '@tauri-apps/api/app'
import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { createI18n } from 'vue-i18n'
import { SettingsAbout } from '@/components/settings/about'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { enUS } from '@/lang/enUS'
import { version as buildVersion } from '../../package.json'

jest.mock('@tauri-apps/api/app', () => ({ getVersion: jest.fn() }))
jest.mock('@/composables/useAppUpdater', () => ({ useAppUpdater: jest.fn() }))
jest.mock('@/components/ui/button', () => ({ Button: { template: '<button><slot /></button>' } }))
jest.mock('@/components/ui/card', () => Object.fromEntries(
  ['Card', 'CardContent', 'CardDescription', 'CardHeader', 'CardTitle']
    .map(name => [name, { template: '<div><slot /></div>' }]),
))

const updater = {
  checkForUpdates: jest.fn(),
  downloadAndInstall: jest.fn(),
  isChecking: ref(false),
  isDownloading: ref(false),
  isInstalling: ref(false),
  isRestarting: ref(false),
  updateAvailable: ref(false),
  updateInfo: ref<{ version: string } | null>(null),
  downloadProgress: ref<number | null>(null),
  updateError: ref(''),
  isUpToDate: ref(false),
}
const render = () => mount(SettingsAbout, {
  global: { plugins: [createI18n({ legacy: false, locale: 'enUS', messages: { enUS } })] },
})

beforeEach(() => {
  jest.clearAllMocks()
  jest.mocked(getVersion).mockResolvedValue('2.3.4')
  jest.mocked(useAppUpdater).mockReturnValue(updater as unknown as ReturnType<typeof useAppUpdater>)
  updater.isChecking.value = false
  updater.isDownloading.value = false
  updater.isInstalling.value = false
  updater.isRestarting.value = false
  updater.updateAvailable.value = false
  updater.updateInfo.value = null
  updater.updateError.value = ''
  updater.isUpToDate.value = false
})

it('shows the installed app version and checks automatically without installing', async () => {
  const wrapper = render()
  await flushPromises()
  expect(wrapper.text()).toContain('2.3.4')
  expect(updater.checkForUpdates).toHaveBeenCalledWith(false, { includeSkipped: true })
  expect(updater.downloadAndInstall).not.toHaveBeenCalled()
  expect(wrapper.text()).not.toContain('You have the latest version')
  expect(wrapper.findAll('a').map(link => link.attributes('href'))).toEqual([
    'https://github.com/geek-fun/sqlkit',
    'https://www.geekfun.club',
  ])
  wrapper.unmount()
})

it('uses the package version when native version lookup is unavailable', async () => {
  jest.mocked(getVersion).mockRejectedValueOnce(new Error('Browser preview'))
  const wrapper = render()
  await flushPromises()
  expect(wrapper.text()).toContain(buildVersion)
  expect(wrapper.text()).not.toContain('0.8.7')
  wrapper.unmount()
})

it('keeps update discovery separate from the explicit install action', async () => {
  const wrapper = render()
  await wrapper.get('button').trigger('click')
  expect(updater.checkForUpdates).toHaveBeenLastCalledWith(true, { includeSkipped: true })
  updater.updateAvailable.value = true
  updater.updateInfo.value = { version: '2.4.0' }
  await flushPromises()
  expect(wrapper.text()).toContain('Version 2.4.0 is available')
  expect(wrapper.get('button').text()).toBe('Update Now')
  expect(updater.downloadAndInstall).not.toHaveBeenCalled()
  await wrapper.get('button').trigger('click')
  expect(updater.downloadAndInstall).toHaveBeenCalledTimes(1)
  wrapper.unmount()
})

it('shows current, pending, and error states without claiming success after failure', async () => {
  const wrapper = render()
  updater.isUpToDate.value = true
  await flushPromises()
  expect(wrapper.text()).toContain('You have the latest version')
  updater.isChecking.value = true
  await flushPromises()
  expect(wrapper.get('button').attributes('disabled')).toBeDefined()
  expect(wrapper.find('[role="progressbar"]').exists()).toBe(true)
  updater.isChecking.value = false
  updater.isUpToDate.value = false
  updater.updateError.value = 'Network unavailable'
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('Network unavailable')
  expect(wrapper.text()).not.toContain('You have the latest version')
  expect(wrapper.get('button').attributes('disabled')).toBeUndefined()
  wrapper.unmount()
})
