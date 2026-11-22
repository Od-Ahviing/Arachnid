import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import HomeView from '../../src/views/HomeView.vue'
import { useInstallation } from '../../src/stores/installation'
import { selectedLanguage } from '../../src/composables/i18n'

const api = vi.hoisted(() => ({
  launchStatus: vi.fn(),
  startGame: vi.fn(),
}))
vi.mock('../../src/services/desktop', () => ({
  desktop: api,
  desktopAvailable: true,
  errorCode: (error: { code?: string }) => error.code ?? 'unknownError',
}))

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  selectedLanguage.value = 'en'
})

describe('launch consent', () => {
  it('shows tracked time on Home and keeps Play available when tracking is unavailable', async () => {
    const store = useInstallation()
    store.report = {
      recognized: true,
      root: 'synthetic',
      fileCount: 0,
      pkzCount: 0,
      totalBytes: 0,
      scannedAt: 0,
    } as typeof store.report
    api.launchStatus.mockResolvedValue({
      decision: { ready: true, reasons: [] },
      session: { state: 'idle' },
      playtimeSeconds: 7260,
    })
    const options = { global: { stubs: { RouterLink: true } } }
    const view = mount(HomeView, options)
    await flushPromises()
    expect(view.get('.home-hero .playtime-summary').text()).toContain('2 hr 1 min')
    expect(view.findAll('.playtime-summary')).toHaveLength(1)
    expect(view.get('.playtime-summary').text()).not.toContain('since')
    view.unmount()
    api.launchStatus.mockResolvedValue({
      decision: { ready: true, reasons: [] },
      session: { state: 'idle' },
      playtimeSeconds: null,
    })
    const unavailable = mount(HomeView, options)
    await flushPromises()
    expect(unavailable.get('.playtime-summary').text()).toContain('Play time is unavailable')
    expect(unavailable.get('.play-section button').attributes('disabled')).toBeUndefined()
    unavailable.unmount()
  })
  it('shows ReShade advice for a returned session and remembers its dismissal across navigation', async () => {
    const store = useInstallation()
    store.report = {
      recognized: true,
      root: 'synthetic',
      fileCount: 0,
      pkzCount: 0,
      totalBytes: 0,
      scannedAt: 0,
    } as typeof store.report
    api.launchStatus.mockResolvedValue({
      decision: { ready: true, reasons: [] },
      session: { state: 'exited', pid: 42, exitCode: -1073741819 },
      graphicsCrashAdvice: 'reshade',
    })
    const mountHome = () =>
      mount(HomeView, {
        global: {
          stubs: { RouterLink: true, ActionDialog: { template: '<dialog><slot /></dialog>' } },
        },
      })
    const view = mountHome()
    await flushPromises()
    expect(view.text()).toContain('Disable ReShade in Mods')
    await view.get('dialog button').trigger('click')
    expect(view.find('dialog').exists()).toBe(false)
    view.unmount()
    const returned = mountHome()
    await flushPromises()
    expect(returned.find('dialog').exists()).toBe(false)
    returned.unmount()
  })

  it('does not launch on mount and treats two play clicks as one pending request', async () => {
    const store = useInstallation()
    store.report = {
      recognized: true,
      root: 'synthetic',
      fileCount: 0,
      pkzCount: 0,
      totalBytes: 0,
      scannedAt: 0,
    } as typeof store.report
    api.launchStatus.mockResolvedValue({
      decision: { ready: true, reasons: [] },
      session: { state: 'idle' },
    })
    let finish!: (value: unknown) => void
    api.startGame.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve
        }),
    )
    const view = mount(HomeView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(api.startGame).not.toHaveBeenCalled()
    await view.get('button').trigger('click')
    await view.get('button').trigger('click')
    expect(api.startGame).toHaveBeenCalledExactlyOnceWith('en', expect.any(Function))
    finish({ decision: { ready: false, reasons: [] }, session: { state: 'running' } })
    await flushPromises()
    expect(view.text()).toContain('Closing Arachnid leaves the game open')
    view.unmount()
  })

  it('rechecks a blocked installation on Play and displays the new diagnostic report on failure', async () => {
    const store = useInstallation()
    store.report = {
      recognized: true,
      root: 'synthetic',
      fileCount: 0,
      pkzCount: 0,
      totalBytes: 0,
      scannedAt: 0,
      checks: [],
    } as unknown as NonNullable<typeof store.report>
    const blocked = {
      decision: { ready: false, reasons: ['localDirect3dUnverified'] },
      session: { state: 'idle' },
    }
    api.launchStatus.mockResolvedValue(blocked)
    api.startGame.mockImplementation(async (_language, onReport) => {
      onReport({ ...store.report, scannedAt: 42 })
      throw { code: 'localDirect3dUnverified' }
    })
    const view = mount(HomeView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    await view.get('button').trigger('click')
    await flushPromises()
    expect(api.startGame).toHaveBeenCalledOnce()
    expect(store.report?.scannedAt).toBe(42)
    expect(store.error).toBe('localDirect3dUnverified')
    view.unmount()
  })
})
