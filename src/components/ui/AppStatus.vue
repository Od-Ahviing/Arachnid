<script setup lang="ts">
import { CircleCheck, CircleAlert, CircleHelp, CircleMinus } from '@lucide/vue'
import { t } from '../../composables/i18n'
import type { CheckStatus } from '../../types/installation'
defineProps<{ status: CheckStatus }>()
const icons = {
  passed: CircleCheck,
  warning: CircleAlert,
  error: CircleAlert,
  unverified: CircleHelp,
  skipped: CircleMinus,
}
</script>

<template>
  <span class="app-status" :class="`app-status--${status}`">
    <component :is="icons[status]" :size="15" aria-hidden="true" />{{ t(status) }}
  </span>
</template>
