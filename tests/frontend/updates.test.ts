import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import AppUpdateBanner from '../../src/components/layout/AppUpdateBanner.vue'
import { selectedLanguage } from '../../src/composables/i18n'

const api = vi.hoisted(() => ({ checkAppUpdate: vi.fn(), installAppUpdate: vi.fn() }))
vi.mock('../../src/services/desktop', () => ({
  desktop: api,
  errorCode: (error: { code: string }) => error.code,
}))

afterEach(() => {
  vi.resetAllMocks()
  selectedLanguage.value = 'auto'
})

describe('launcher updates', () => {
  it('keeps the banner hidden when no update exists or the feed is unavailable', async () => {
    api.checkAppUpdate.mockResolvedValueOnce(null).mockRejectedValueOnce(new Error('offline'))
    for (let attempt = 0; attempt < 2; attempt++) {
      const banner = mount(AppUpdateBanner, { props: { visible: true, canInstall: true } })
      await flushPromises()
      expect(banner.find('section').exists()).toBe(false)
      banner.unmount()
    }
  })

  it('waits for approval and lets the player dismiss the banner', async () => {
    selectedLanguage.value = 'fr'
    api.checkAppUpdate.mockResolvedValue('0.1.2')
    const banner = mount(AppUpdateBanner, { props: { visible: false, canInstall: false } })
    await flushPromises()
    expect(banner.find('section').exists()).toBe(false)
    await banner.setProps({ visible: true })
    expect(banner.text()).toContain('Mise à jour disponible · 0.1.2')
    expect(banner.get('button').attributes('disabled')).toBeDefined()
    await banner.findAll('button')[1].trigger('click')
    expect(api.installAppUpdate).not.toHaveBeenCalled()
    expect(banner.find('section').exists()).toBe(false)
    banner.unmount()
  })

  it('shows download progress and offers a retry after a rejected installation', async () => {
    selectedLanguage.value = 'en'
    api.checkAppUpdate.mockResolvedValue('0.1.2')
    let reject!: (error: { code: string }) => void
    api.installAppUpdate.mockImplementation((_, progress) => {
      progress({ received: 50, total: 100 })
      return new Promise((_resolve, rejectDownload) => {
        reject = rejectDownload
      })
    })
    const banner = mount(AppUpdateBanner, { props: { visible: true, canInstall: true } })
    await flushPromises()
    await banner.get('button').trigger('click')
    expect(api.installAppUpdate).toHaveBeenCalledWith('0.1.2', expect.any(Function))
    expect(banner.get('progress').attributes('value')).toBe('50')
    expect(banner.find('button').exists()).toBe(false)
    reject({ code: 'appUpdateDownloadFailed' })
    await flushPromises()
    expect(banner.get('[role="alert"]').text()).toContain('signature verification failed')
    expect(banner.get('button').text()).toBe('Update now')
    banner.unmount()
  })
})
