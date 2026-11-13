<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch, useId } from 'vue'
const props = defineProps<{ open: boolean; title: string; busy: boolean }>()
const emit = defineEmits<{ close: [] }>()
const dialog = ref<HTMLDialogElement>()
const titleId = useId()
watch(
  () => props.open,
  async (open) => {
    await nextTick()
    if (open && !dialog.value?.open) dialog.value?.showModal()
    else if (!open && dialog.value?.open) dialog.value.close()
  },
  { immediate: true },
)
onBeforeUnmount(() => dialog.value?.close())
</script>
<template>
  <dialog
    ref="dialog"
    class="action-dialog"
    :aria-labelledby="titleId"
    :aria-busy="busy || undefined"
    @cancel.prevent="!busy && emit('close')"
  >
    <h2 :id="titleId">{{ title }}</h2>
    <slot />
  </dialog>
</template>
<style scoped>
.action-dialog {
  color: var(--color-text-primary);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-dialog);
  padding: 1.75rem;
  width: min(36rem, calc(100vw - 2rem));
  max-height: calc(100dvh - 3rem);
  overflow: auto;
  margin: auto;
}
.action-dialog::backdrop {
  background: rgb(0 0 0 / 70%);
}
h2 {
  font-size: var(--text-section);
  margin-bottom: 1rem;
}
</style>
