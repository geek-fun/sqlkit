import { assembleSystemPrompt, buildAgentModeInstructions } from './agentPrompt'

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

describe('agent mode contract', () => {
  it.each([false, true])('keeps reads automatic and all deletes and DDL subject to application approval (auto=%s)', (auto) => {
    const prompt = buildAgentModeInstructions(auto).join('\n')
    expect(prompt).toContain('read-only queries and metadata reads execute automatically in both modes')
    expect(prompt).toContain('CREATE, ALTER, DROP, DELETE with or without WHERE, and TRUNCATE')
    expect(prompt).toContain('saved confirmation rules')
    expect(prompt).toContain('Approval does not grant a denied permission')
    expect(prompt).toContain('Do not substitute a conversational approval question')
  })

  it('requires approval for normal writes in Ask and executes them automatically in Auto', () => {
    expect(buildAgentModeInstructions(false).join('\n')).toContain('inserts and updates require approval in ASK')
    expect(buildAgentModeInstructions(true).join('\n')).toContain('inserts and updates execute automatically in AUTO')
  })
})
