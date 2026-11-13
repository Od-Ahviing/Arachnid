<script setup lang="ts">
import AppStatus from '../ui/AppStatus.vue'
import { t } from '../../composables/i18n'
import type { Check } from '../../types/installation'
defineProps<{ checks: Check[] }>()
</script>

<template>
  <ul class="check-list">
    <li v-for="(check, index) in checks" :key="`${check.code}-${index}`" class="check-row">
      <div>
        <h3>{{ t(check.code) }}</h3>
        <p>{{ t(`${check.code}Hint`) }}</p>
        <code v-if="check.value" class="check-value">{{ check.value }}</code>
      </div>
      <AppStatus :status="check.status" />
    </li>
  </ul>
</template>
