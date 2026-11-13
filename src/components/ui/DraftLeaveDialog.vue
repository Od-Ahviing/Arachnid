<script setup lang="ts">
import { ui } from '../../composables/interfaceMessages'
import ActionDialog from './ActionDialog.vue'
import AppButton from './AppButton.vue'
defineProps<{ open: boolean; busy: boolean; error?: string; details?: string[] }>()
defineEmits<{ apply: []; discard: []; stay: [] }>()
</script>
<template>
  <ActionDialog :open="open" :title="ui('changes')" :busy="busy" @close="$emit('stay')">
    <p>{{ ui('leaveHint') }}</p>
    <ul v-if="details?.length" class="change-summary">
      <li v-for="detail in details" :key="detail">{{ detail }}</li>
    </ul>
    <p v-if="error" role="alert" class="notice notice--error">{{ error }}</p>
    <div class="dialog-actions">
      <AppButton variant="quiet" :disabled="busy" @click="$emit('stay')">{{
        ui('stay')
      }}</AppButton>
      <AppButton :disabled="busy" @click="$emit('discard')">{{ ui('discard') }}</AppButton>
      <AppButton variant="primary" :loading="busy" @click="$emit('apply')">{{
        ui('apply')
      }}</AppButton>
    </div>
  </ActionDialog>
</template>
