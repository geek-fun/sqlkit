import { toast, useNotifications } from '@/composables/useNotifications'

jest.useFakeTimers()

describe('useNotifications', () => {
  beforeEach(() => {
    jest.clearAllMocks()
    jest.clearAllTimers()
    const { toasts, dismiss } = useNotifications()
    toasts.value.forEach(t => dismiss(t.id))
  })

  describe('toast', () => {
    it('adds success toast', () => {
      toast.success('Operation successful')

      const { toasts } = useNotifications()
      expect(toasts.value).toHaveLength(1)
      expect(toasts.value[0].type).toBe('success')
      expect(toasts.value[0].title).toBe('Operation successful')
    })

    it('adds error toast', () => {
      toast.error('Something went wrong')

      const { toasts } = useNotifications()
      expect(toasts.value).toHaveLength(1)
      expect(toasts.value[0].type).toBe('error')
    })

    it('adds info toast', () => {
      toast.info('For your information')

      const { toasts } = useNotifications()
      expect(toasts.value).toHaveLength(1)
      expect(toasts.value[0].type).toBe('info')
    })

    it('adds toast with description', () => {
      toast.success('Success', { description: 'Detailed message' })

      const { toasts } = useNotifications()
      expect(toasts.value[0].description).toBe('Detailed message')
    })

    it('generates unique ids for each toast', () => {
      toast.success('First')
      toast.success('Second')
      toast.success('Third')

      const { toasts } = useNotifications()
      const ids = toasts.value.map(t => t.id)
      expect(new Set(ids).size).toBe(3)
    })

    it('auto-dismisses after duration', () => {
      toast.success('Auto-dismiss')

      const { toasts } = useNotifications()
      expect(toasts.value).toHaveLength(1)

      jest.advanceTimersByTime(4000)

      expect(toasts.value).toHaveLength(0)
    })
  })

  describe('dismiss', () => {
    it('removes toast by id', () => {
      toast.success('Test')
      const { toasts, dismiss } = useNotifications()
      const id = toasts.value[0].id

      dismiss(id)

      expect(toasts.value).toHaveLength(0)
    })

    it('does nothing for non-existent id', () => {
      const { toasts, dismiss } = useNotifications()

      dismiss(999)

      expect(toasts.value).toHaveLength(0)
    })

    it('removes only the specified toast', () => {
      toast.success('First')
      toast.success('Second')
      toast.success('Third')

      const { toasts, dismiss } = useNotifications()
      const idToRemove = toasts.value[1].id

      dismiss(idToRemove)

      expect(toasts.value).toHaveLength(2)
      expect(toasts.value.map(t => t.title)).toEqual(['First', 'Third'])
    })
  })

  describe('toasts reactivity', () => {
    it('toasts array is reactive', () => {
      const { toasts } = useNotifications()

      expect(toasts.value).toHaveLength(0)

      toast.success('One')
      expect(toasts.value).toHaveLength(1)

      toast.success('Two')
      expect(toasts.value).toHaveLength(2)
    })
  })
})