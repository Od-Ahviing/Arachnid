import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import GameLanguageControl from '../../src/components/game/GameLanguageControl.vue'
import { selectedLanguage } from '../../src/composables/i18n'
import { useInstallation } from '../../src/stores/installation'
import type { ScanReport } from '../../src/types/installation'

const api = vi.hoisted(() => ({ saveLanguage: vi.fn(), synchronizeGameLanguage: vi.fn() }))
vi.mock('../../src/services/desktop', () => ({
  desktop: api,
  desktopAvailable: true,
  errorCode: (failure: { code?: string }) => failure.code ?? 'unknownError',
}))
beforeEach(() => {
  vi.resetAllMocks()
  setActivePinia(createPinia())
  selectedLanguage.value = 'en'
  useInstallation().report = { recognized: true, root: 'synthetic' } as ScanReport
  api.saveLanguage.mockResolvedValue(undefined)
  api.synchronizeGameLanguage.mockImplementation(async (language: string) => ({
    requested: language,
    applied: language,
    changed: true,
  }))
})
afterEach(() => {
  selectedLanguage.value = 'auto'
  vi.unstubAllGlobals()
})
describe('shared language control on Home', () => {
  it('saves one choice for the interface and game and prevents concurrent changes', async () => {
    let finish!: () => void
    api.saveLanguage.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve
        }),
    )
    const view = mount(GameLanguageControl)
    expect(view.get('label').text()).toContain('Interface and game')
    await view.get('select').setValue('fr')
    expect(view.get('select').attributes('disabled')).toBeDefined()
    expect(api.synchronizeGameLanguage).not.toHaveBeenCalled()
    finish()
    await flushPromises()
    expect(selectedLanguage.value).toBe('fr')
    expect(api.synchronizeGameLanguage).toHaveBeenCalledExactlyOnceWith('fr')
    expect(view.text()).toContain('Le jeu utilise la langue de l’interface')
    view.unmount()
  })
  it('shows the resolved automatic choice and uses that same language for the game', async () => {
    vi.stubGlobal('navigator', { languages: ['fr-CA'] })
    const view = mount(GameLanguageControl)
    await view.get('select').setValue('auto')
    await flushPromises()
    expect(view.get('option[value="auto"]').text()).toContain('Français')
    expect(api.saveLanguage).toHaveBeenCalledExactlyOnceWith('auto')
    expect(api.synchronizeGameLanguage).toHaveBeenCalledExactlyOnceWith('fr')
    view.unmount()
  })
})
