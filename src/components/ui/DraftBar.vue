<script setup lang="ts">
import { ui } from '../../composables/interfaceMessages'
import AppButton from './AppButton.vue'
defineProps<{ dirty: boolean; busy: boolean; details?: string[] }>()
defineEmits<{ apply: []; discard: [] }>()
</script>
<template>
  <div v-if="dirty || busy" class="draft-bar">
    <div role="status">
      <strong>{{ ui(busy ? 'working' : 'changes') }}</strong>
      <ul v-if="details?.length" class="change-summary">
        <li v-for="detail in details" :key="detail">{{ detail }}</li>
      </ul>
    </div>
    <div class="draft-bar-actions">
      <AppButton variant="quiet" :disabled="busy" @click="$emit('discard')">{{
        ui('discard')
      }}</AppButton>
      <AppButton variant="primary" :loading="busy" @click="$emit('apply')">{{
        ui('apply')
      }}</AppButton>
    </div>
  </div>
</template>
