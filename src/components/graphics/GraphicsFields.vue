<script setup lang="ts">
import type { SettingsReport } from '../../types/operations'
import { t } from '../../composables/i18n'
import { ui } from '../../composables/interfaceMessages'
const props = defineProps<{
  fields: SettingsReport['settings']
  modelValue: Record<string, number | null>
  disabled: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: Record<string, number | null>] }>()
function change(name: string, event: Event) {
  const value = (event.target as HTMLInputElement | HTMLSelectElement).value
  emit('update:modelValue', { ...props.modelValue, [name]: value === '' ? null : Number(value) })
}
function options(name: string) {
  if (name === 'ShadowDetail') return [1, 2, 3, 4]
  if (['LightShafts', 'DrawDistance'].includes(name)) return [1, 2, 3]
  if (['Ssao', 'Particles'].includes(name)) return [0, 1, 3]
  if (name === 'MaxAniso') return [0, 2, 4, 8, 16]
  if (name === 'AntiAliasing') return [0, 1, 3]
  return [0, 1]
}
function optionLabel(name: string, value: number) {
  if (name === 'Halo') return value === 0 ? t('lowOption') : t('highOption')
  if (['ShadowDetail', 'LightShafts', 'DrawDistance'].includes(name))
    return t(
      ['lowOption', 'mediumOption', 'highOption', 'veryHighOption'][value - 1] ?? 'settingOpaque',
    )
  if (['Ssao', 'Particles'].includes(name))
    return value === 0 ? t('disabledOption') : value === 1 ? t('lowOption') : t('highOption')
  if (name === 'MaxAniso') return value === 0 ? t('disabledOption') : `${value}×`
  if (name === 'AntiAliasing')
    return value === 0 ? t('disabledOption') : value === 1 ? t('lowOption') : t('highOption')
  return value === 0 ? t('disabledOption') : t('enabledOption')
}
</script>
<template>
  <div v-if="fields.some((field) => ['Width', 'Height'].includes(field.name))" class="settings-row">
    <span id="resolution-label">{{ ui('resolution') }}</span>
    <div class="resolution-controls" role="group" aria-labelledby="resolution-label">
      <label
        v-for="field in fields.filter((field) => ['Width', 'Height'].includes(field.name))"
        :key="field.name"
        :for="'setting-' + field.name"
      >
        <span>{{ ui(field.name === 'Width' ? 'width' : 'height') }}</span>
        <input
          :id="'setting-' + field.name"
          :value="modelValue[field.name] ?? ''"
          type="number"
          step="1"
          :min="field.name === 'Width' ? 640 : 480"
          :max="field.name === 'Width' ? 7680 : 4320"
          :disabled="disabled"
          required
          @input="change(field.name, $event)"
        />
      </label>
    </div>
  </div>
  <div
    v-for="setting in fields.filter((field) => !['Width', 'Height'].includes(field.name))"
    :key="setting.name"
    class="settings-row"
  >
    <div>
      <label :for="'setting-' + setting.name">{{
        setting.name === 'Fullscreen' ? ui('displayMode') : t('setting' + setting.name)
      }}</label>
    </div>
    <select
      :id="'setting-' + setting.name"
      :value="modelValue[setting.name]"
      :disabled="disabled"
      @change="change(setting.name, $event)"
    >
      <option v-for="value in options(setting.name)" :key="value" :value="value">
        {{
          setting.name === 'Fullscreen'
            ? ui(value === 1 ? 'fullscreen' : 'windowed')
            : optionLabel(setting.name, value)
        }}
      </option>
    </select>
  </div>
</template>
<style scoped>
.resolution-controls {
  display: flex;
  gap: 0.75rem;
  width: min(100%, 18rem);
}
.resolution-controls label {
  min-width: 0;
  flex: 1;
}
.resolution-controls label span {
  display: block;
  color: var(--color-text-secondary);
  font-size: var(--text-technical);
  margin-bottom: 0.35rem;
}
.resolution-controls input {
  width: 100%;
}
</style>
