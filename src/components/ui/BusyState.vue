<script setup lang="ts">
import { LoaderCircle } from '@lucide/vue'
defineProps<{ label: string }>()
</script>
<template>
  <div class="busy-state" role="status" aria-live="polite">
    <LoaderCircle :size="20" class="spinner" aria-hidden="true" />
    <span>{{ label }}</span>
    <slot />
  </div>
</template>
