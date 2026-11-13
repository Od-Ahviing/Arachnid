import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppButton from '../../src/components/ui/AppButton.vue'
import CheckList from '../../src/components/diagnostics/CheckList.vue'
import { selectedLanguage } from '../../src/composables/i18n'

describe('shared controls', () => {
  it('prevents a second action while loading and preserves the action name', async () => {
    const wrapper = mount(AppButton, { props: { loading: true }, slots: { default: 'Inspect' } })
    expect(wrapper.get('button').attributes('disabled')).toBeDefined()
    expect(wrapper.get('button').attributes('aria-busy')).toBe('true')
    expect(wrapper.text()).toBe('Inspect')
    await wrapper.get('button').trigger('click')
    expect(wrapper.emitted('click')).toBeUndefined()
  })
  it('renders unverified findings as text without a false success', () => {
    selectedLanguage.value = 'en'
    const wrapper = mount(CheckList, {
      props: { checks: [{ code: 'runtimeUntested', status: 'unverified', value: null }] },
    })
    expect(wrapper.text()).toContain('Not verified')
    expect(wrapper.text()).toContain('has not been launched')
    expect(wrapper.text()).not.toContain('Checked')
    selectedLanguage.value = 'auto'
  })
})
