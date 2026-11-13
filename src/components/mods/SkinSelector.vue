<script setup lang="ts">
import { computed } from 'vue'
import { ui } from '../../composables/interfaceMessages'
import { skinText as t } from '../../composables/skinMessages'
import { modsUiText as m } from '../../composables/modsUiMessages'
import type { SkinStatus } from '../../types/mods'
const props = defineProps<{ status: SkinStatus; modelValue: string; disabled: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const appliedName = computed(
  () => props.status.skins.find((skin) => skin.id === props.status.selected)?.name ?? t('original'),
)
</script>
<template>
  <section class="skins">
    <h2>{{ t('title') }}</h2>
    <p class="hint">{{ t('hint') }}</p>
    <div class="applied-costume">
      <span>{{ ui('applied') }}</span
      ><strong>{{ appliedName }}</strong>
    </div>
    <label for="skin-choice">{{ t('label') }}</label>
    <select
      id="skin-choice"
      :value="modelValue"
      :disabled="
        disabled ||
        status.filesChanged ||
        status.recoveryRequired ||
        (!status.available && !status.selected)
      "
      @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option value="">{{ t('original') }}</option>
      <option
        v-for="skin in status.skins"
        :key="skin.id"
        :value="skin.id"
        :disabled="!status.available"
      >
        {{ skin.name }} · {{ skin.author }}
      </option>
    </select>
    <p v-if="status.filesChanged" role="alert">{{ m('conflict') }}</p>
    <p v-else-if="!status.available" role="status">{{ t('unavailable') }}</p>
    <p class="hint">{{ ui('originalHint') }}</p>
    <details class="technical-details">
      <summary>{{ ui('advanced') }}</summary>
      <p>{{ t('testing') }}</p>
    </details>
  </section>
</template>
<style scoped>
.skins {
  padding: 1.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-panel);
  background: var(--color-sidebar);
}
.hint {
  font-size: var(--text-caption);
  margin-top: 0.5rem;
}
label {
  display: block;
  margin-bottom: 0.5rem;
}
select {
  display: block;
  width: 100%;
  margin-bottom: 1rem;
}
</style>
