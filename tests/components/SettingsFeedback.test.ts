/** @jest-environment jsdom */
import { mount } from '@vue/test-utils'
import { SettingsFeedback } from '@/components/settings'

it('unknown progress stays indeterminate rather than inventing a percentage', () => {
  const wrapper = mount(SettingsFeedback, { props: { state: 'pending', label: 'Preparing download…' } })
  const bar = wrapper.get('[role="progressbar"]')
  expect(bar.attributes('aria-valuenow')).toBeUndefined()
  expect(wrapper.text()).not.toContain('%')
  expect(wrapper.get('[role="status"]').text()).toContain('Preparing download')
})

it('clamps real progress and retains meaningful completion and failure states', async () => {
  const wrapper = mount(SettingsFeedback, { props: { state: 'pending', label: 'Downloading…', progress: 47 } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('47')
  await wrapper.setProps({ progress: 120 })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('100')
  await wrapper.setProps({ state: 'success', label: 'Installation complete' })
  expect(wrapper.find('[role="progressbar"]').exists()).toBe(false)
  expect(wrapper.text()).toContain('Installation complete')
  await wrapper.setProps({ state: 'error', label: 'Action failed', detail: 'Offline. Retry.' })
  expect(wrapper.get('[role="alert"]').text()).toContain('Offline. Retry.')
})
