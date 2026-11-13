<script setup lang="ts">
import { LoaderCircle } from '@lucide/vue'
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'quiet'
    loading?: boolean
    disabled?: boolean
    type?: 'button' | 'submit'
  }>(),
  { variant: 'secondary', loading: false, disabled: false, type: 'button' },
)
</script>

<template>
  <button
    :type="type"
    class="app-button"
    :class="`app-button--${variant}`"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
  >
    <LoaderCircle v-if="loading" class="button-spinner" :size="16" aria-hidden="true" />
    <slot />
  </button>
</template>

<style scoped>
.button-spinner {
  animation: button-spin 1s linear infinite;
  flex-shrink: 0;
}
@keyframes button-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .button-spinner {
    animation: none;
  }
}
</style>
