import { assembleSystemPrompt } from './agentPrompt'

describe('assembleSystemPrompt', () => {
  it('joins layers in developer, global, connections, sidebar order', () => {
    expect(assembleSystemPrompt({
      base: 'base',
      userGlobal: 'global',
      connectionPrompts: [{ alias: 'warehouse', prompt: 'warehouse context' }],
      sidebarContext: 'editor context',
    })).toBe('base\n\nglobal\n\nwarehouse context\n\neditor context')
  })

  it('labels each prompt when multiple connection contexts are attached', () => {
    expect(assembleSystemPrompt({
      base: 'base',
      connectionPrompts: [
        { alias: 'warehouse', prompt: 'revenue data' },
        { alias: 'crm', prompt: 'customer data' },
      ],
    })).toBe('base\n\n## Connection Context: warehouse\n\nrevenue data\n\n## Connection Context: crm\n\ncustomer data')
  })

  it('skips blank optional layers', () => {
    expect(assembleSystemPrompt({
      base: 'base',
      userGlobal: '  ',
      connectionPrompts: [{ alias: 'warehouse', prompt: '\n' }],
      sidebarContext: '',
    })).toBe('base')
  })

  it('returns an empty prompt when every layer is blank', () => {
    expect(assembleSystemPrompt({ base: ' ', userGlobal: '\n', sidebarContext: '' })).toBe('')
  })
})
