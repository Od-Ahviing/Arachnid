import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ModCatalog from '../../src/components/mods/ModCatalog.vue'
import { selectedLanguage } from '../../src/composables/i18n'
import type { ModStatus, PackPreview } from '../../src/types/mods'
import catalog from '../../manifests/community-catalog.json'
const api = vi.hoisted(() => ({
  modStatus: vi.fn(),
  preparePack: vi.fn(),
  applyPack: vi.fn(),
  skinStatus: vi.fn(),
}))
vi.mock('../../src/services/desktop', () => ({
  desktop: api,
  errorCode: (error: { code?: string }) => error.code ?? 'unknownError',
}))
const status: ModStatus = {
  defaultComponents: { fix: true, dxvk: false, reshade: false },
  components: { fix: true, dxvk: true, reshade: true },
  catalog,
  selection: { schemaVersion: 1, enabled: ['performance-pack'] },
  activationAvailable: true,
  pack: {
    active: true,
    recoveryRequired: false,
    settingsOutOfSync: false,
    filesChanged: false,
    phase: 'committed',
    requiresUpdate: false,
  },
  preview: null,
}
const preview: PackPreview = {
  digest: 'exact-digest',
  operation: 'enable',
  display: { width: 1920, height: 1080, windowed: false },
  phase: 'prepared',
  files: [],
}
beforeEach(() => {
  vi.resetAllMocks()
  selectedLanguage.value = 'en'
  api.modStatus.mockResolvedValue(status)
  api.preparePack.mockResolvedValue(preview)
  api.applyPack.mockResolvedValue(undefined)
  api.skinStatus.mockResolvedValue({
    skins: [],
    selected: null,
    available: false,
    filesChanged: false,
    recoveryRequired: false,
    preview: null,
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
  selectedLanguage.value = 'auto'
})
describe('independent mod choices', () => {
  it('shows all eight choices and independent controls in all five languages', () => {
    for (const language of ['fr', 'en', 'de', 'it', 'es'] as const) {
      selectedLanguage.value = language
      for (let bits = 0; bits < 8; bits++) {
        const draft = {
          fix: Boolean(bits & 1),
          dxvk: Boolean(bits & 2),
          reshade: Boolean(bits & 4),
        }
        const wrapper = mount(ModCatalog, { props: { status, draft, busy: false } })
        expect(
          wrapper
            .findAll<HTMLInputElement>('input[role="switch"]')
            .map((control) => control.element.checked),
        ).toEqual([draft.fix, draft.dxvk, draft.reshade])
        expect(wrapper.text()).toContain('Fix Enhancers')
        wrapper.unmount()
      }
    }
  })
  it('emits a component choice without mutating the authoritative state', async () => {
    const wrapper = mount(ModCatalog, { props: { status, draft: status.components, busy: false } })
    await wrapper.find('input').setValue(false)
    expect(wrapper.emitted('change')).toEqual([['fix', false]])
    expect(status.components.fix).toBe(true)
    await wrapper.setProps({ busy: true })
    expect(
      wrapper.findAll('input').every((control) => control.attributes('disabled') !== undefined),
    ).toBe(true)
  })
})
