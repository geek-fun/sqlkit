/** @jest-environment jsdom */
import type { AgentToolCall } from '@/store/dataStudioStore'
import { mount } from '@vue/test-utils'
import { enUS } from '@/lang/enUS'
import ToolConfirmationCard from '@/views/data-studio/components/tool-confirmation-card.vue'

jest.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => {
      const value = key.split('.').reduce<unknown>(
        (acc, part) => (acc && typeof acc === 'object'
          ? (acc as Record<string, unknown>)[part]
          : undefined),
        enUS,
      )
      return typeof value === 'string'
        ? value
        : key
    },
  }),
}))

jest.mock('@/store/appStore', () => ({
  ThemeType: { AUTO: 'auto', LIGHT: 'light', DARK: 'dark' },
  useAppStore: () => ({ uiThemeType: 'light' }),
}))

jest.mock('@/components/ui/badge', () => ({
  Badge: { template: '<span class="mock-badge"><slot /></span>' },
}))

jest.mock('@/components/ui/button', () => ({
  Button: { template: '<button type="button" v-bind="$attrs"><slot /></button>' },
}))

describe('toolConfirmationCard', () => {
  it('renders formatted highlighted SQL without escaped quotes for the screenshot payload', () => {
    const sql = [
      'INSERT INTO store.orders',
      '  (order_no, shipping_addr)',
      'VALUES',
      '(\'SO-20260400007\', \'{"recipient":"User One"}\'::jsonb);',
    ].join('\n')

    const toolCall: AgentToolCall = {
      id: 'tc-1',
      toolName: 'sqlkit__execute_write',
      args: { connection_id: 'pg-bastion-password', sql },
      status: 'pending',
      riskLevel: 'elevated',
      requiresConfirmation: true,
    }

    const wrapper = mount(ToolConfirmationCard, {
      props: { toolCall },
    })
    const text = wrapper.text()

    expect(text).toContain('Elevated Risk')
    expect(text).toContain('Modifies data — review before allowing')
    expect(text).toContain('pg-bastion-password')
    expect(text).toContain('INSERT')
    expect(text).toContain('store.orders')
    expect(text).toContain('"recipient":"User One"')
    expect(text).not.toContain('\\"connection_id\\"')
    expect(text).not.toContain('\\n')
    expect(wrapper.find('code.hljs').html()).toContain('hljs-keyword')
  })

  it('shows destructive risk, impact hints, and keeps Always Allow available', async () => {
    const toolCall: AgentToolCall = {
      id: 'tc-2',
      toolName: 'sqlkit__execute_delete',
      args: {
        connection_id: 'prod-db',
        sql: 'DELETE FROM store.orders',
      },
      status: 'pending',
      riskLevel: 'destructive',
      requiresConfirmation: true,
    }

    const wrapper = mount(ToolConfirmationCard, {
      props: { toolCall },
    })
    const text = wrapper.text()

    expect(text).toContain('Destructive Operation')
    expect(text).toContain('Deletes data or alters schema — may be irreversible')
    expect(text).toContain('No WHERE clause detected')

    const actionLabels = wrapper.findAll('button').map(btn => btn.text().trim())
    expect(actionLabels).toContain('Allow Once')
    expect(actionLabels).toContain('Always Allow')
    expect(actionLabels).toContain('Deny')
    expect(actionLabels).toContain('Always Deny')
    expect(actionLabels).toContain('Cancel')

    const allowAlwaysBtn = wrapper.findAll('button').find(btn => btn.text().trim() === 'Always Allow')!
    await allowAlwaysBtn.trigger('click')
    expect(wrapper.emitted('confirm')?.[0]).toEqual([
      { toolCallId: 'tc-2', action: 'allow_always' },
    ])
  })

  it('toggles Show all / Show less for long statements', async () => {
    const longSql = Array.from({ length: 15 }, (_, index) => `SELECT ${index};`).join('\n')
    const toolCall: AgentToolCall = {
      id: 'tc-3',
      toolName: 'sqlkit__execute_write',
      args: { connection_id: 'dev', sql: longSql },
      status: 'pending',
      riskLevel: 'elevated',
      requiresConfirmation: true,
    }

    const wrapper = mount(ToolConfirmationCard, {
      props: { toolCall },
    })
    const pre = wrapper.get('pre')
    expect(pre.classes()).toContain('max-h-60')

    const toggleBtn = wrapper.findAll('button').find(btn => btn.text().includes('Show all'))!
    await toggleBtn.trigger('click')
    expect(pre.classes()).toContain('max-h-[480px]')
    expect(wrapper.text()).toContain('Show less')
  })
})
