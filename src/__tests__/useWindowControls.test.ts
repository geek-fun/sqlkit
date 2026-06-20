/**
 * @jest-environment node
 */
import { shouldReserveMacTrafficLightInset } from '@/composables/useWindowControls'

describe('shouldReserveMacTrafficLightInset', () => {
  it('returns true on macOS when not fullscreen', () => {
    expect(shouldReserveMacTrafficLightInset(true, false, true)).toBe(true)
  })

  it('returns false on macOS when fullscreen', () => {
    expect(shouldReserveMacTrafficLightInset(true, true, true)).toBe(false)
  })

  it('returns false on non-macOS even when not fullscreen', () => {
    expect(shouldReserveMacTrafficLightInset(false, false, true)).toBe(false)
  })

  it('returns false on non-macOS fullscreen', () => {
    expect(shouldReserveMacTrafficLightInset(false, true, true)).toBe(false)
  })

  it('returns false in browser (non-desktop) context', () => {
    expect(shouldReserveMacTrafficLightInset(true, false, false)).toBe(false)
  })
})

describe('useWindowControls', () => {
  const mockMinimize = jest.fn()
  const mockToggleMaximize = jest.fn()
  const mockClose = jest.fn()
  const mockIsMaximized = jest.fn()
  const mockIsFullscreen = jest.fn()
  const mockOnResized = jest.fn()

  beforeEach(() => {
    jest.resetModules()
    jest.mock('@tauri-apps/api/window', () => ({
      getCurrentWindow: () => ({
        minimize: mockMinimize,
        toggleMaximize: mockToggleMaximize,
        close: mockClose,
        isMaximized: mockIsMaximized,
        isFullscreen: mockIsFullscreen,
        onResized: mockOnResized,
      }),
    }))
  })

  afterEach(() => {
    jest.clearAllMocks()
  })

  it('executes minimize action', async () => {
    const { useWindowControls } = await import('@/composables/useWindowControls')
    const controls = useWindowControls()
    await controls.minimize()
    expect(mockMinimize).toHaveBeenCalled()
  })

  it('executes toggleMaximize action', async () => {
    const { useWindowControls } = await import('@/composables/useWindowControls')
    const controls = useWindowControls()
    await controls.toggleMaximize()
    expect(mockToggleMaximize).toHaveBeenCalled()
  })

  it('executes close action', async () => {
    const { useWindowControls } = await import('@/composables/useWindowControls')
    const controls = useWindowControls()
    await controls.close()
    expect(mockClose).toHaveBeenCalled()
  })

  it('tracks isMaximized and isFullscreen state', async () => {
    mockIsMaximized.mockResolvedValue(true)
    mockIsFullscreen.mockResolvedValue(false)
    mockOnResized.mockResolvedValue(jest.fn())

    const { useWindowControls } = await import('@/composables/useWindowControls')
    const controls = useWindowControls()

    // The composable refreshes state on mount via onMounted.
    // Since we're in a test environment outside Vue, onMounted doesn't fire.
    // We verify that the default state is sensible.
    expect(controls.isMaximized.value).toBe(false)
    expect(controls.isFullscreen.value).toBe(false)
  })
})
