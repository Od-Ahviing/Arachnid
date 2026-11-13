<script setup lang="ts">
import { ShieldCheck, Layers, Sparkles } from '@lucide/vue'
import type { ModComponents, ModStatus } from '../../types/mods'
import { modsUiText as m } from '../../composables/modsUiMessages'
import { ui } from '../../composables/interfaceMessages'
const props = defineProps<{ status: ModStatus; draft: ModComponents; busy: boolean }>()
const emit = defineEmits<{ change: [key: keyof ModComponents, enabled: boolean] }>()
const cards = [
  { key: 'fix', hint: 'fixHint', icon: ShieldCheck, id: 'fix-enhancers' },
  { key: 'dxvk', hint: 'dxvkHint', icon: Layers, id: 'dxvk' },
  { key: 'reshade', hint: 'reshadeHint', icon: Sparkles, id: 'reshade' },
] as const
function change(key: keyof ModComponents, event: Event) {
  const input = event.target as HTMLInputElement
  const checked = input.checked
  input.checked = props.draft[key]
  emit('change', key, checked)
}
</script>

<template>
  <div class="mod-cards">
    <section
      v-for="card in cards"
      :key="card.key"
      class="mod-card"
      :class="{ selected: draft[card.key] }"
    >
      <div class="mod-card-heading">
        <component :is="card.icon" :size="22" aria-hidden="true" />
        <h3 :id="`mod-${card.key}`">{{ m(card.key) }}</h3>
        <label class="mod-switch">
          <input
            type="checkbox"
            role="switch"
            :aria-labelledby="`mod-${card.key}`"
            :checked="draft[card.key]"
            :disabled="busy"
            @change="change(card.key, $event)"
          />
          <span aria-hidden="true" />
        </label>
      </div>
      <p>{{ m(card.hint) }}</p>
      <span
        class="mod-state"
        :class="{ pending: draft[card.key] !== status.components[card.key] }"
        >{{
          draft[card.key] !== status.components[card.key]
            ? ui(draft[card.key] ? 'willEnable' : 'willDisable')
            : m(status.components[card.key] ? 'active' : 'inactive')
        }}</span
      >
      <details class="technical-details">
        <summary>{{ m('sources') }}</summary>
        <template
          v-for="entry in status.catalog.entries.filter((entry) => entry.id === card.id)"
          :key="entry.id"
        >
          <p>{{ entry.author }}</p>
          <code>{{ entry.source }}</code>
          <p v-if="entry.license">{{ entry.license.name }}</p>
        </template>
      </details>
    </section>
  </div>
</template>

<style scoped>
.mod-cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1rem;
}
.mod-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.25rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  transition: border-color 0.2s;
}
.mod-card.selected {
  border-color: var(--color-primary);
}
.mod-card-heading {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
h3 {
  font-family: inherit;
  font-size: 1rem;
  margin: 0;
}
.mod-card p {
  color: var(--color-text-secondary);
  font-size: 0.875rem;
  line-height: 1.65;
  margin: 0;
}
.mod-state {
  margin-top: auto;
  font-size: 0.8rem;
  color: var(--color-text-secondary);
}
.mod-state.pending {
  color: var(--color-warning);
}
.mod-switch {
  margin-left: auto;
  position: relative;
  width: 2.5rem;
  height: 1.5rem;
  flex-shrink: 0;
}
.mod-switch input {
  position: absolute;
  inset: 0;
  opacity: 0;
  width: 100%;
  height: 100%;
  margin: 0;
  cursor: pointer;
}
.mod-switch span {
  display: block;
  width: 100%;
  height: 100%;
  border-radius: 2rem;
  background: var(--color-border);
  pointer-events: none;
  transition: background-color 0.2s;
}
.mod-switch span::after {
  content: '';
  display: block;
  position: absolute;
  top: 0.2rem;
  left: 0.2rem;
  height: 1.1rem;
  width: 1.1rem;
  border-radius: 50%;
  background: var(--color-text-primary);
  transition: transform 0.2s;
}
.mod-switch input:checked + span {
  background: var(--color-primary);
}
.mod-switch input:checked + span::after {
  transform: translateX(1rem);
}
.mod-switch input:focus-visible + span {
  outline: 2px solid var(--color-text-primary);
  outline-offset: 3px;
}
.mod-switch input:disabled + span {
  opacity: 0.5;
}
details {
  font-size: 0.8rem;
}
code {
  overflow-wrap: anywhere;
}
@media (max-width: 1000px) {
  .mod-cards {
    grid-template-columns: 1fr;
  }
}
@media (prefers-reduced-motion: reduce) {
  .mod-card,
  .mod-switch span,
  .mod-switch span::after {
    transition: none;
  }
}
</style>
