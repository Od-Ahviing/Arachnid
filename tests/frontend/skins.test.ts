import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SkinSelector from '../../src/components/mods/SkinSelector.vue'
import type { SkinStatus } from '../../src/types/mods'
const status: SkinStatus = {
  skins: [{ id: 'arachnoman', name: 'ArachnoMan', author: 'Sosiska' }],
  selected: null,
  available: true,
  filesChanged: false,
  recoveryRequired: false,
  requiresUpdate: false,
  preview: null,
}
describe('costume draft', () => {
  it('emits choices without changing the applied costume or performing I/O', async () => {
    const view = mount(SkinSelector, { props: { status, modelValue: '', disabled: false } })
    await view.get('select').setValue('arachnoman')
    expect(view.emitted('update:modelValue')).toEqual([['arachnoman']])
    expect(status.selected).toBeNull()
    await view.setProps({ modelValue: 'arachnoman' })
    await view.get('select').setValue('')
    expect(view.emitted('update:modelValue')?.[1]).toEqual([''])
  })
  it('blocks conflicting files and choices during application', async () => {
    const view = mount(SkinSelector, { props: { status, modelValue: '', disabled: true } })
    expect(view.get('select').attributes('disabled')).toBeDefined()
    await view.setProps({ disabled: false, status: { ...status, filesChanged: true } })
    expect(view.get('select').attributes('disabled')).toBeDefined()
  })
})
