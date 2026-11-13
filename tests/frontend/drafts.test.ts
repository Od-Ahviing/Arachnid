import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import GraphicsView from '../../src/views/GraphicsView.vue'
import ModsView from '../../src/views/ModsView.vue'
import RepairView from '../../src/views/RepairView.vue'
import App from '../../src/App.vue'
import { useInstallation } from '../../src/stores/installation'
import { selectedLanguage } from '../../src/composables/i18n'
import { interfaceMessages } from '../../src/composables/interfaceMessages'
import type { ScanReport } from '../../src/types/installation'
import type { ModComponents } from '../../src/types/mods'
import catalog from '../../manifests/community-catalog.json'

const api = vi.hoisted(() => ({
  nativeSettings: vi.fn(),
  settingsOperation: vi.fn(),
  prepareSettings: vi.fn(),
  prepareProfile: vi.fn(),
  applySettings: vi.fn(),
  restoreSettings: vi.fn(),
  modStatus: vi.fn(),
  preparePack: vi.fn(),
  applyPack: vi.fn(),
  skinStatus: vi.fn(),
  prepareSkin: vi.fn(),
  applySkin: vi.fn(),
  scan: vi.fn(),
  onProgress: vi.fn(),
}))
vi.mock('../../src/services/desktop', () => ({
  desktop: api,
  desktopAvailable: true,
  errorCode: (error: { code?: string }) => error.code ?? 'unknownError',
}))
const installation = {
  root: 'synthetic',
  recognized: true,
  checks: [],
  binaries: [],
  files: [],
  fileCount: 0,
  pkzCount: 0,
  totalBytes: 0,
  scannedAt: 0,
} as unknown as ScanReport
const initialSettings = {
  settings: [
    { name: 'Width', section: 'Display', value: 1920, valueType: 4, editable: true },
    { name: 'Height', section: 'Display', value: 1080, valueType: 4, editable: true },
    { name: 'VSync', section: 'Display', value: 0, valueType: 4, editable: true },
    { name: 'AntiAliasing', section: 'Display', value: null, valueType: null, editable: false },
    { name: 'Language', section: 'Install', value: null, valueType: null, editable: true },
  ],
  profiles: [{ name: 'low', available: true, settings: [{ name: 'VSync', value: 1 }] }],
  gameLanguages: [
    { code: 'en', nativeName: 'English', value: 1, available: true },
    { code: 'fr', nativeName: 'Français', value: 2, available: true },
    { code: 'de', nativeName: 'Deutsch', value: 4, available: false },
  ],
  currentProfile: 'custom',
  runtimeQualified: true,
  capabilities: [],
}
let components: ModComponents
let chosenComponents: ModComponents
let skin: string | null
let chosenSkin: string | null
const wrappers: VueWrapper[] = []
function button(view: VueWrapper, name: string) {
  const result = view.findAll('button').find((control) => control.text() === name)
  if (!result) throw new Error(`Missing button: ${name}`)
  return result
}
function settingsView() {
  const view = mount(GraphicsView, { attachTo: document.body })
  wrappers.push(view)
  return view
}
function modsView() {
  const view = mount(ModsView)
  wrappers.push(view)
  return view
}
async function routedView(page: '/graphics' | '/mods') {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/graphics', component: GraphicsView },
      { path: '/mods', component: ModsView },
      { path: '/home', component: { template: '<h1>Home destination</h1>' } },
    ],
  })
  await router.push(page)
  const view = mount(defineComponent({ template: '<RouterView />' }), {
    attachTo: document.body,
    global: { plugins: [router] },
  })
  wrappers.push(view)
  await flushPromises()
  return { view, router }
}
beforeEach(() => {
  vi.resetAllMocks()
  setActivePinia(createPinia())
  useInstallation().report = { ...installation }
  selectedLanguage.value = 'en'
  components = { fix: true, dxvk: true, reshade: true }
  chosenComponents = { ...components }
  skin = null
  chosenSkin = null
  api.nativeSettings.mockResolvedValue(structuredClone(initialSettings))
  api.settingsOperation.mockResolvedValue(null)
  api.prepareSettings.mockResolvedValue({ digest: 'settings-digest', plan: { actions: [] } })
  api.applySettings.mockResolvedValue(undefined)
  api.restoreSettings.mockResolvedValue(undefined)
  api.onProgress.mockResolvedValue(() => undefined)
  api.scan.mockResolvedValue(installation)
  api.modStatus.mockImplementation(async () => ({
    defaultComponents: { fix: true, dxvk: false, reshade: false },
    components: { ...components },
    catalog,
    selection: { enabled: [] },
    activationAvailable: true,
    pack: {
      active: true,
      recoveryRequired: false,
      settingsOutOfSync: false,
      filesChanged: false,
      requiresUpdate: false,
      phase: 'committed',
    },
    preview: null,
  }))
  api.skinStatus.mockImplementation(async () => ({
    skins: [{ id: 'arachnoman', name: 'ArachnoMan', author: 'Sosiska' }],
    selected: skin,
    available: true,
    filesChanged: false,
    requiresUpdate: false,
    recoveryRequired: false,
    preview: null,
  }))
  api.preparePack.mockImplementation(async (_, choices) => {
    chosenComponents = { ...choices }
    return { digest: 'pack-digest', files: [] }
  })
  api.applyPack.mockImplementation(async () => {
    components = { ...chosenComponents }
    return installation
  })
  api.prepareSkin.mockImplementation(async (selected) => {
    chosenSkin = selected
    return { digest: 'skin-digest', files: [] }
  })
  api.applySkin.mockImplementation(async () => {
    skin = chosenSkin
  })
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value() {
      this.open = true
    },
  })
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value() {
      this.open = false
    },
  })
})
afterEach(() => {
  wrappers.splice(0).forEach((view) => view.unmount())
  selectedLanguage.value = 'auto'
})

describe('deferred graphics changes', () => {
  it('keeps a cleared resolution pending and prevents applying an incomplete resolution', async () => {
    const view = settingsView()
    await flushPromises()
    await view.get('#setting-Width').setValue('')
    expect(view.find('.draft-bar').exists()).toBe(true)
    expect(view.get('.draft-bar').text()).toContain('Resolution width')
    await button(view, 'Apply changes').trigger('click')
    expect(api.prepareSettings).not.toHaveBeenCalled()
    await button(view, 'Discard changes').trigger('click')
    expect(view.get<HTMLInputElement>('#setting-Width').element.value).toBe('1920')
    expect(view.find('.draft-bar').exists()).toBe(false)
  })
  it('offers the five official presets and marks an adjusted preset as custom', async () => {
    api.nativeSettings.mockResolvedValueOnce({
      ...structuredClone(initialSettings),
      profiles: ['veryLow', 'low', 'medium', 'high', 'amazing'].map((name) => ({
        name,
        available: true,
        settings: [{ name: 'VSync', value: 1 }],
      })),
    })
    const view = settingsView()
    await flushPromises()
    for (const name of ['Very low', 'Low', 'Medium', 'High', 'Amazing']) {
      expect(button(view, name).exists()).toBe(true)
    }
    await button(view, 'Amazing').trigger('click')
    expect(button(view, 'Amazing').attributes('aria-pressed')).toBe('true')
    expect(api.prepareSettings).not.toHaveBeenCalled()
    await view.get('#setting-VSync').setValue('0')
    expect(view.get('.profile-panel h2').text()).toContain('Custom')
    expect(button(view, 'Amazing').attributes('aria-pressed')).toBe('false')
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(api.prepareSettings).toHaveBeenCalledExactlyOnceWith(
      [{ name: 'VSync', value: 0 }],
      'amazing',
    )
  })
  it('offers a read-only retry after settings fail to load', async () => {
    api.nativeSettings.mockRejectedValueOnce({ code: 'fileUnreadable' })
    const view = settingsView()
    await flushPromises()
    expect(view.find('#setting-Width').exists()).toBe(false)
    await button(view, 'Try again').trigger('click')
    await flushPromises()
    expect(view.find('#setting-Width').exists()).toBe(true)
    expect(api.prepareSettings).not.toHaveBeenCalled()
  })
  it('accumulates rapid edits and applies the final choices once, then scans the directory', async () => {
    const view = settingsView()
    await flushPromises()
    await view.get('#setting-Width').setValue(1280)
    await view.get('#setting-Height').setValue(720)
    await view.get('#setting-Width').setValue(1600)
    expect(api.prepareSettings).not.toHaveBeenCalled()
    expect(view.get('#setting-Height').attributes('disabled')).toBeUndefined()
    expect(view.find('.draft-bar').exists()).toBe(true)
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(api.prepareSettings).toHaveBeenCalledExactlyOnceWith(
      [
        { name: 'Width', value: 1600 },
        { name: 'Height', value: 720 },
      ],
      undefined,
    )
    expect(api.applySettings).toHaveBeenCalledExactlyOnceWith('settings-digest')
    expect(api.scan).toHaveBeenCalledExactlyOnceWith('synthetic')
    expect(api.scan.mock.invocationCallOrder[0]).toBeGreaterThan(
      api.applySettings.mock.invocationCallOrder[0]!,
    )
  })
  it('selects profiles locally, preserves resolution, and includes subsequent overrides in one plan', async () => {
    const view = settingsView()
    await flushPromises()
    await view.get('#setting-Width').setValue(1280)
    await button(view, 'Low').trigger('click')
    expect(api.prepareProfile).not.toHaveBeenCalled()
    expect(api.prepareSettings).not.toHaveBeenCalled()
    expect(view.get<HTMLInputElement>('#setting-VSync').element.value).toBe('1')
    await view.get('#setting-VSync').setValue('0')
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(api.prepareSettings).toHaveBeenCalledWith(
      [
        { name: 'Width', value: 1280 },
        { name: 'VSync', value: 0 },
      ],
      'low',
    )
  })
  it('keeps absent graphics settings read-only and removes the competing game-language control', async () => {
    const view = settingsView()
    await flushPromises()
    expect(view.get('#setting-AntiAliasing').element.tagName).toBe('DD')
    expect(view.find('#setting-Language').exists()).toBe(false)
    expect(view.find('.draft-bar').exists()).toBe(false)
    expect(api.prepareSettings).not.toHaveBeenCalled()
  })
  it('keeps failed changes on the page and does not claim success when verification fails', async () => {
    const view = settingsView()
    await flushPromises()
    await view.get('#setting-Width').setValue(1280)
    api.prepareSettings.mockRejectedValueOnce({ code: 'stalePlan' })
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(view.get<HTMLInputElement>('#setting-Width').element.value).toBe('1280')
    expect(api.applySettings).not.toHaveBeenCalled()
    api.scan.mockRejectedValueOnce({ code: 'fileUnreadable' })
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(view.text()).toContain('installation check did not finish')
    expect(view.text()).not.toContain('Values written and read back')
  })
})

describe('deferred mod and costume changes', () => {
  it.each(['enable', 'disable'] as const)(
    'publishes the native verification for %s on Mods without a second scan',
    async (operation) => {
      const verifiedReport = { ...installation, scannedAt: 1234 }
      api.applyPack.mockImplementationOnce(async () => {
        components = { ...chosenComponents }
        return verifiedReport
      })
      const { view, router } = await routedView('/mods')
      if (operation === 'disable') {
        for (const control of view.findAll('input[role="switch"]')) await control.setValue(false)
      } else {
        await view.findAll('input[role="switch"]')[2]!.setValue(false)
      }
      await button(view, 'Apply changes').trigger('click')
      await flushPromises()
      expect(api.applyPack).toHaveBeenCalledExactlyOnceWith('pack-digest', false)
      expect(api.preparePack.mock.calls[0]?.[0]).toBe(operation)
      expect(api.scan).not.toHaveBeenCalled()
      expect(useInstallation().report?.scannedAt).toBe(1234)
      expect(view.text()).toContain('Changes applied and checked')
      expect(router.currentRoute.value.path).toBe('/mods')
      await router.push('/home')
      expect(router.currentRoute.value.path).toBe('/home')
    },
  )
  it('retries a busy verification before reading the committed mod state', async () => {
    api.applyPack.mockImplementationOnce(async () => {
      components = { ...chosenComponents }
      return null
    })
    api.scan.mockRejectedValueOnce({ code: 'installationBusy' })
    const view = modsView()
    await flushPromises()
    await view.findAll('input[role="switch"]')[2]!.setValue(false)
    await button(view, 'Apply changes').trigger('click')
    await vi.waitFor(() => expect(view.text()).toContain('Changes applied and checked'))
    expect(api.applyPack).toHaveBeenCalledTimes(1)
    expect(api.scan).toHaveBeenCalledTimes(2)
    expect(api.modStatus.mock.invocationCallOrder.at(-1)).toBeGreaterThan(
      api.scan.mock.invocationCallOrder.at(-1)!,
    )
  })
  it('shows an automatically applied Fix without creating a new draft', async () => {
    components = { fix: true, dxvk: false, reshade: false }
    const view = modsView()
    await flushPromises()
    expect(
      view.findAll<HTMLInputElement>('input[role="switch"]').map((input) => input.element.checked),
    ).toEqual([true, false, false])
    expect(view.find('.draft-bar').exists()).toBe(false)
    expect(api.preparePack).not.toHaveBeenCalled()
  })
  it('preserves an installed trio and lets defaults be selected without applying them', async () => {
    const view = modsView()
    await flushPromises()
    expect(
      view
        .findAll<HTMLInputElement>('input[role="switch"]')
        .every((input) => input.element.checked),
    ).toBe(true)
    expect(view.find('.draft-bar').exists()).toBe(false)
    await button(view, 'Default selection').trigger('click')
    expect(view.get('.draft-bar').text()).toContain('DXVK · Will be disabled')
    expect(view.get('.draft-bar').text()).toContain('ReShade · Will be disabled')
    expect(api.preparePack).not.toHaveBeenCalled()
    await button(view, 'Discard changes').trigger('click')
    expect(
      view
        .findAll<HTMLInputElement>('input[role="switch"]')
        .every((input) => input.element.checked),
    ).toBe(true)
  })
  it('allows several graphics and costume choices without preparation and applies only the final selection', async () => {
    const view = modsView()
    await flushPromises()
    const switches = view.findAll('input[role="switch"]')
    await switches[0]!.setValue(false)
    await switches[1]!.setValue(false)
    await switches[0]!.setValue(true)
    await view.get('#skin-choice').setValue('arachnoman')
    expect(api.preparePack).not.toHaveBeenCalled()
    expect(api.prepareSkin).not.toHaveBeenCalled()
    await button(view, 'Apply changes').trigger('click')
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(api.preparePack).toHaveBeenCalledExactlyOnceWith('enable', {
      fix: true,
      dxvk: false,
      reshade: true,
    })
    expect(api.prepareSkin).toHaveBeenCalledExactlyOnceWith('arachnoman')
    expect(api.applyPack).toHaveBeenCalledExactlyOnceWith('pack-digest', false)
    expect(api.applySkin).toHaveBeenCalledExactlyOnceWith('skin-digest', false)
    expect(api.scan).toHaveBeenCalledExactlyOnceWith('synthetic')
    expect(view.text()).toContain('Changes applied and checked')
  })
  it('retains unapplied choices after a partial failure and does not reapply the successful component', async () => {
    const view = modsView()
    await flushPromises()
    await view.find('input').setValue(false)
    await view.get('#skin-choice').setValue('arachnoman')
    api.applySkin.mockRejectedValueOnce({ code: 'fileChanged' })
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(view.find('.draft-bar').exists()).toBe(true)
    expect(view.get<HTMLSelectElement>('#skin-choice').element.value).toBe('arachnoman')
    expect(view.text()).not.toContain('Changes applied and checked')
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(api.applyPack).toHaveBeenCalledTimes(1)
    expect(api.applySkin).toHaveBeenCalledTimes(2)
  })
  it('rejects conflicts before writing files', async () => {
    api.preparePack.mockResolvedValueOnce({ digest: 'conflict', files: [{ conflict: true }] })
    const view = modsView()
    await flushPromises()
    await view.find('input').setValue(false)
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(api.applyPack).not.toHaveBeenCalled()
    expect(view.find('.draft-bar').exists()).toBe(true)
  })
})

describe('leaving with pending choices', () => {
  it('retries a saved installation after a startup error without making the user pick it again', async () => {
    const store = useInstallation()
    store.report = null
    store.path = 'synthetic'
    const initialize = vi.spyOn(store, 'initialize').mockImplementation(async () => {
      store.initializing = false
      store.error = 'installationBusy'
    })
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/setup', component: { template: '<h1>Pick a folder</h1>' } },
        { path: '/home', component: { template: '<h1>Ready</h1>' } },
      ],
    })
    await router.push('/setup')
    const view = mount(App, { global: { plugins: [router] } })
    wrappers.push(view)
    await flushPromises()
    expect(view.find('.startup-retry').exists()).toBe(true)
    expect(view.text()).toContain('synthetic')
    expect(view.text()).not.toContain('Pick a folder')
    initialize.mockImplementationOnce(async () => {
      store.initializing = false
      store.error = null
      store.report = installation
    })
    await button(view, 'Try again').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/home')
    expect(view.find('.startup-retry').exists()).toBe(false)
    expect(view.text()).toContain('Ready')
  })
  it('keeps unverified diagnostic results separate from warnings and successful checks', async () => {
    useInstallation().report = {
      ...installation,
      checks: [
        { code: 'direct3d', status: 'passed', value: null },
        { code: 'wrapper', status: 'warning', value: null },
        { code: 'runtimeUntested', status: 'unverified', value: null },
      ],
    }
    const view = mount(RepairView)
    wrappers.push(view)
    expect(view.findAll('.check-row')).toHaveLength(1)
    await button(view, 'Not verified 1').trigger('click')
    expect(view.findAll('.check-row')).toHaveLength(1)
    expect(view.text()).toContain('has not been launched')
    expect(view.text()).not.toContain('Direct3D 9 runtime')
  })
  it('retries a failed mod scan without applying the successful transaction again', async () => {
    api.applyPack.mockImplementationOnce(async () => {
      components = { ...chosenComponents }
      return null
    })
    api.scan.mockRejectedValueOnce({ code: 'fileUnreadable' })
    const view = modsView()
    await flushPromises()
    await view.find('input').setValue(false)
    await button(view, 'Apply changes').trigger('click')
    await flushPromises()
    expect(view.text()).not.toContain('Changes applied and checked')
    await button(view, 'Try again').trigger('click')
    await flushPromises()
    expect(api.applyPack).toHaveBeenCalledOnce()
    expect(api.scan).toHaveBeenCalledTimes(2)
    expect(view.text()).toContain('Changes applied and checked')
  })
  it('returns to an invalid graphics field instead of trapping the user behind the leave dialog', async () => {
    const { view, router } = await routedView('/graphics')
    await view.get('#setting-Width').setValue(1)
    const navigation = router.push('/home')
    await flushPromises()
    await view
      .get('dialog[open]')
      .findAll('button')
      .find((control) => control.text() === 'Apply changes')!
      .trigger('click')
    await navigation
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/graphics')
    expect(view.find('dialog[open]').exists()).toBe(false)
    expect(view.get<HTMLInputElement>('#setting-Width').element.validity.rangeUnderflow).toBe(true)
    expect(api.prepareSettings).not.toHaveBeenCalled()
  })
  it('shows the animated brand only while application initialization is pending', async () => {
    const store = useInstallation()
    store.initializing = true
    let initialized: () => void = () => undefined
    vi.spyOn(store, 'initialize').mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          initialized = () => {
            store.initializing = false
            resolve()
          }
        }),
    )
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/home', component: { template: '<h1>Launcher ready</h1>' } }],
    })
    await router.push('/home')
    const view = mount(App, { global: { plugins: [router] } })
    wrappers.push(view)
    expect(view.find('.startup-mark img').exists()).toBe(true)
    expect(view.get('.startup-mark img').attributes('src')).toContain('arachnid-loading.gif')
    expect(view.get('.startup-mark source').attributes('media')).toBe(
      '(prefers-reduced-motion: reduce)',
    )
    expect(view.get('.startup-mark source').attributes('srcset')).toContain('arachnid-logo.png')
    expect(view.find('.startup-screen[aria-busy="true"]').exists()).toBe(true)
    initialized()
    await flushPromises()
    expect(view.find('.startup-screen').exists()).toBe(false)
    expect(view.text()).toContain('Launcher ready')
  })
  it('updates the page title with the language and focuses the heading only after navigation', async () => {
    const store = useInstallation()
    vi.spyOn(store, 'initialize').mockImplementation(async () => {
      store.initializing = false
    })
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/home', component: { template: '<h1 tabindex="-1">Home</h1>' } },
        { path: '/settings', component: { template: '<h1 tabindex="-1">Settings</h1>' } },
      ],
    })
    await router.push('/home')
    const view = mount(App, { attachTo: document.body, global: { plugins: [router] } })
    wrappers.push(view)
    await flushPromises()
    expect(document.activeElement).toBe(view.get('h1').element)
    expect(document.title).toBe('Home · Arachnid')
    selectedLanguage.value = 'fr'
    await flushPromises()
    expect(document.title).toBe('Accueil · Arachnid')
    await router.push('/settings')
    await flushPromises()
    expect(document.activeElement).toBe(view.get('h1').element)
    expect(document.title).toBe('Paramètres · Arachnid')
  })
  it.each(['/graphics', '/mods'] as const)(
    'can stay, discard, or apply before leaving %s',
    async (page) => {
      const { view, router } = await routedView(page)
      if (page === '/graphics') await view.get('#setting-Width').setValue(1280)
      else await view.find('input').setValue(false)
      let navigation = router.push('/home')
      await flushPromises()
      expect(router.currentRoute.value.path).toBe(page)
      expect(view.get('dialog[open]').text()).toContain('Apply your choices')
      await button(view, 'Stay on this page').trigger('click')
      await navigation
      expect(router.currentRoute.value.path).toBe(page)
      navigation = router.push('/home')
      await flushPromises()
      await view
        .get('dialog[open]')
        .findAll('button')
        .find((control) => control.text() === 'Apply changes')!
        .trigger('click')
      await navigation
      await flushPromises()
      expect(router.currentRoute.value.path).toBe('/home')
      expect(api.scan).toHaveBeenCalledTimes(page === '/graphics' ? 1 : 0)
    },
  )
  it('discards choices without any mutation when leaving', async () => {
    const { view, router } = await routedView('/mods')
    await view.find('input').setValue(false)
    const navigation = router.push('/home')
    await flushPromises()
    await view
      .get('dialog[open]')
      .findAll('button')
      .find((control) => control.text() === 'Discard changes')!
      .trigger('click')
    await navigation
    expect(router.currentRoute.value.path).toBe('/home')
    expect(api.preparePack).not.toHaveBeenCalled()
    expect(api.applyPack).not.toHaveBeenCalled()
  })
  it('keeps the navigation blocked after a failed apply', async () => {
    const { view, router } = await routedView('/mods')
    await view.find('input').setValue(false)
    void router.push('/home')
    await flushPromises()
    api.applyPack.mockRejectedValueOnce({ code: 'fileChanged' })
    await view
      .get('dialog[open]')
      .findAll('button')
      .find((control) => control.text() === 'Apply changes')!
      .trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/mods')
    expect(view.find('dialog[open]').exists()).toBe(true)
  })
  it('has complete translated action labels in all five languages', () => {
    const keys = Object.keys(interfaceMessages.en)
    for (const dictionary of Object.values(interfaceMessages)) {
      expect(Object.keys(dictionary).sort()).toEqual([...keys].sort())
      expect(Object.values(dictionary).every((value) => value.length > 0)).toBe(true)
    }
  })
})
