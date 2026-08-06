/**
 * @jest-environment node
 */
import { analyzeCompletionContext } from '@/composables/sqlCompletion/analyzer'

describe('analyzeCompletionContext', () => {
  it('(a) detects the word and active table after FROM', () => {
    const text = 'SELECT * FROM users WHERE u'
    const ctx = analyzeCompletionContext(text, text.length)
    expect(ctx.word).toBe('u')
    expect(ctx.activeTable?.table).toBe('users')
    expect(ctx.tableRefs).toEqual([{ table: 'users' }])
    expect(ctx.inComment).toBe(false)
  })

  it('(b) detects alias-qualified column word', () => {
    const text = 'SELECT u.na FROM users u'
    const ctx = analyzeCompletionContext(text, 'SELECT u.na'.length)
    expect(ctx.word).toBe('na')
    expect(ctx.qualifier).toBe('u.')
    expect(ctx.isAfterDot).toBe(true)
    expect(ctx.activeTable?.table).toBe('users')
    expect(ctx.activeTable?.alias).toBe('u')
  })

  it('(c) handles quoted table names with spaces', () => {
    const text = 'SELECT * FROM "my table" WHERE '
    const ctx = analyzeCompletionContext(text, text.length)
    expect(ctx.activeTable?.table).toBe('my table')
  })

  it('(d) handles schema-qualified table with alias', () => {
    const text = 'SELECT * FROM public.orders o WHERE o.'
    const ctx = analyzeCompletionContext(text, text.length)
    expect(ctx.qualifier).toBe('o.')
    expect(ctx.activeTable?.table).toBe('orders')
    expect(ctx.activeTable?.schema).toBe('public')
  })

  it('(e) ignores FROM inside a line comment', () => {
    const text = '-- FROM foo\nSELECT 1'
    const ctx = analyzeCompletionContext(text, text.length)
    expect(ctx.tableRefs).toEqual([])
    expect(ctx.activeTable).toBeNull()
  })

  it('(f) last JOIN table wins for its own alias', () => {
    const text = 'SELECT * FROM a JOIN b ON a.id=b.id WHERE b.'
    const ctx = analyzeCompletionContext(text, text.length)
    expect(ctx.qualifier).toBe('b.')
    expect(ctx.activeTable?.table).toBe('b')
    expect(ctx.tableRefs.length).toBe(2)
  })

  it('(g) offset inside a block comment yields no table refs', () => {
    const text = 'SELECT * FROM a /* FROM x */'
    const idx = text.indexOf('/* FROM x */') + 5
    const ctx = analyzeCompletionContext(text, idx)
    expect(ctx.inComment).toBe(true)
    expect(ctx.tableRefs).toEqual([])
  })

  it('(h) empty string → empty context, no throw', () => {
    const ctx = analyzeCompletionContext('', 0)
    expect(ctx.word).toBe('')
    expect(ctx.tableRefs).toEqual([])
    expect(ctx.activeTable).toBeNull()
    expect(ctx.inComment).toBe(false)
  })

  it('supports AS-alias and multi-statement document', () => {
    const text = 'SELECT * FROM users AS u;\nSELECT o.id FROM orders o'
    const ctx = analyzeCompletionContext(text, text.length)
    expect(ctx.activeTable?.table).toBe('orders')
    expect(ctx.activeTable?.alias).toBe('o')
  })

  it('word completion on empty word at end of FROM still yields the table ref', () => {
    const text = 'SELECT * FROM users '
    const ctx = analyzeCompletionContext(text, text.length)
    expect(ctx.word).toBe('')
    expect(ctx.activeTable?.table).toBe('users')
  })
})
