import { cn } from '@/lib/utils'

describe('semantic font sizes', () => {
  it.each(['text-caption', 'text-body'])(
    'overrides the default size with %s while preserving the text color',
    (size) => {
      const classes = cn('text-sm font-medium', `${size} text-muted-foreground`).split(' ')

      expect(classes).toEqual(expect.arrayContaining([size, 'text-muted-foreground', 'font-medium']))
      expect(classes).not.toContain('text-sm')
    },
  )

  it.each(['text-caption', 'text-body'])(
    'preserves status colors when %s follows a component variant',
    (size) => {
      const classes = cn('text-xs text-destructive-text bg-destructive-muted', size).split(' ')

      expect(classes).toEqual(expect.arrayContaining([size, 'text-destructive-text', 'bg-destructive-muted']))
      expect(classes).not.toContain('text-xs')
    },
  )

  it('allows a standard size to override a semantic size', () => {
    expect(cn('text-caption text-primary-text', 'text-sm')).toBe('text-primary-text text-sm')
  })

  it('merges sizes within the same responsive variant independently of color', () => {
    expect(cn('md:text-caption md:text-destructive-text', 'md:text-body'))
      .toBe('md:text-destructive-text md:text-body')
  })
})
