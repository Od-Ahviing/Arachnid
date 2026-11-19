import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import SavesView from '../../src/views/SavesView.vue'
import { selectedLanguage, messages } from '../../src/composables/i18n'
import type { SaveStatus } from '../../src/types/saves'

const api = vi.hoisted(() => ({
  saveStatus: vi.fn(),
  detectSaveSource: vi.fn(),
  openSaveFolder: vi.fn(),
  deleteSave: vi.fn(),
  chooseSaveSource: vi.fn(),
  configureSaveguard: vi.fn(),
  snapshotSaves: vi.fn(),
  pinSave: vi.fn(),
  prepareSaveRestore: vi.fn(),
  applySaveRestore: vi.fn(),
}))
vi.mock('../../src/services/desktop', () => ({
  desktop: api,
  errorCode: (e: { code?: string }) => e.code || 'unknownError',
}))

const initial: SaveStatus = {
  backupDirectory: 'C:\\LocalAppData\\Arachnid\\saveguard',
  profile: {
    schemaVersion: 1,
    id: 'profile',
    source: 'C:\\Disposable\\Save',
    enabled: true,
    keep: 20,
    quotaBytes: 268435456,
  },
  snapshots: [
    {
      id: 'snapshot-original',
      profileId: 'profile',
      createdAt: 1794230000,
      pinned: false,
      reason: 'manual',
      status: 'copyVerified',
      files: [{ path: 'slot.dat', bytes: 3, sha256: 'hash' }],
    },
  ],
  operation: null,
  error: null,
  busy: false,
}
let wrappers: ReturnType<typeof mount>[] = []
beforeEach(() => {
  vi.resetAllMocks()
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.setAttribute('open', '')
    },
  })
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.removeAttribute('open')
    },
  })
  selectedLanguage.value = 'en'
  api.saveStatus.mockResolvedValue(structuredClone(initial))
})
afterEach(() => {
  wrappers.forEach((w) => w.unmount())
  wrappers = []
  vi.restoreAllMocks()
})
async function view() {
  const wrapper = mount(SavesView)
  wrappers.push(wrapper)
  await flushPromises()
  return wrapper
}

describe('SaveGuard automatic copies and guided actions', () => {
  it('displays and saves a one-GiB storage quota', async () => {
    const status = structuredClone(initial)
    status.profile!.quotaBytes = 1024 * 1024 * 1024
    api.saveStatus.mockResolvedValue(status)
    const wrapper = await view()
    expect((wrapper.get('input[max="1024"]').element as HTMLInputElement).value).toBe('1024')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(api.configureSaveguard).toHaveBeenCalledWith(true, 20, 1024)
  })
  it('uses automatic detection and a fixed backup folder without a picker on this page', async () => {
    const wrapper = await view()
    expect(wrapper.text()).toContain('Automatic protection is on')
    expect(wrapper.text()).toContain('Open backup folder')
    expect(api.chooseSaveSource).not.toHaveBeenCalled()
    expect(wrapper.text()).not.toContain('Choose save folder')
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Open backup folder')!
      .trigger('click')
    await flushPromises()
    expect(api.openSaveFolder).toHaveBeenCalledOnce()
  })
  it('guides restore through acknowledgement, exact preview and a single approval', async () => {
    const operation = {
      digest: 'exact-digest',
      phase: 'prepared',
      files: [
        { path: 'slot.dat', beforeBytes: 3, afterBytes: 4, beforeHash: 'old', afterHash: 'new' },
      ],
    }
    api.prepareSaveRestore.mockResolvedValue(operation)
    const wrapper = await view()
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Restore')!
      .trigger('click')
    await flushPromises()
    const review = () => wrapper.findAll('button').find((b) => b.text() === 'Review changes')!
    expect(review().attributes('disabled')).toBeDefined()
    await wrapper.find('dialog input[type="checkbox"]').setValue(true)
    await review().trigger('click')
    await flushPromises()
    expect(api.prepareSaveRestore).toHaveBeenCalledWith('snapshot-original', true)
    expect(api.applySaveRestore).not.toHaveBeenCalled()
    const approve = wrapper.findAll('button').find((b) => b.text() === 'Restore this copy')!
    expect(wrapper.text()).toContain('exact-digest')
    let release: () => void = () => {}
    api.applySaveRestore.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          release = resolve
        }),
    )
    await approve.trigger('click')
    await approve.trigger('click')
    expect(api.applySaveRestore).toHaveBeenCalledTimes(1)
    expect(api.applySaveRestore).toHaveBeenCalledWith('exact-digest', true, false)
    release()
    await flushPromises()
  })
  it('resumes a durable preview and resets the producer acknowledgement', async () => {
    api.saveStatus.mockResolvedValue({
      ...initial,
      operation: { digest: 'exact-digest', phase: 'prepared', files: [] },
    })
    const wrapper = await view()
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Continue restoration')!
      .trigger('click')
    await flushPromises()
    expect(
      wrapper
        .findAll('button')
        .find((b) => b.text() === 'Restore this copy')!
        .attributes('disabled'),
    ).toBeDefined()
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Cancel')!
      .trigger('click')
    await flushPromises()
    expect(api.applySaveRestore).toHaveBeenCalledWith('exact-digest', false, true)
  })
  it('requires confirmation to delete a copy and protects the last copy', async () => {
    const another = { ...initial.snapshots[0]!, id: 'another-copy' }
    api.saveStatus.mockResolvedValue({ ...initial, snapshots: [...initial.snapshots, another] })
    const wrapper = await view()
    await wrapper
      .findAll('button')
      .find((b) => b.attributes('aria-label')?.startsWith('Delete'))!
      .trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('Delete this backup?')
    expect(api.deleteSave).not.toHaveBeenCalled()
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Delete')!
      .trigger('click')
    await flushPromises()
    expect(api.deleteSave).toHaveBeenCalledWith('snapshot-original')
    expect(api.applySaveRestore).not.toHaveBeenCalled()
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Undo deletion')!
      .trigger('click')
    await flushPromises()
    expect(api.deleteSave).toHaveBeenLastCalledWith('snapshot-original', true)
  })
  it('disables deletion for the final copy', async () => {
    const wrapper = await view()
    expect(
      wrapper
        .findAll('button')
        .find((b) => b.attributes('aria-label')?.startsWith('Delete'))!
        .attributes('disabled'),
    ).toBeDefined()
  })
  it('translates every SaveGuard key in all five supported languages', () => {
    for (const language of Object.values(messages))
      expect(Object.keys(language).sort()).toEqual(Object.keys(messages.en).sort())
  })
})
