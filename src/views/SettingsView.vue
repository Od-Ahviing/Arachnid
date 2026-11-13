<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useInstallation } from '../stores/installation'
import { t } from '../composables/i18n'
import AppButton from '../components/ui/AppButton.vue'
import { desktop, errorCode } from '../services/desktop'
import { ui } from '../composables/interfaceMessages'
const source = ref('')
const sourceBusy = ref(false)
const sourceError = ref('')
async function loadSource() {
  try {
    source.value = (await desktop.saveStatus()).profile?.source ?? ''
  } catch (failure) {
    sourceError.value = errorCode(failure)
  }
}
async function changeSource(automatic: boolean) {
  if (sourceBusy.value) return
  sourceBusy.value = true
  sourceError.value = ''
  try {
    if (automatic) await desktop.detectSaveSource(true)
    else await desktop.chooseSaveSource()
    await loadSource()
  } catch (failure) {
    sourceError.value = errorCode(failure)
  } finally {
    sourceBusy.value = false
  }
}
onMounted(loadSource)
const store = useInstallation()
const router = useRouter()
</script>

<template>
  <div class="page">
    <header class="page-heading">
      <p class="eyebrow">{{ t('settings') }}</p>
      <h1 tabindex="-1">{{ t('settingsTitle') }}</h1>
      <p class="intro">{{ t('settingsDescription') }}</p>
    </header>

    <section class="settings-panel">
      <h2 class="panel-title">{{ ui('installation') }}</h2>
      <div class="settings-row">
        <div>
          <h2>{{ t('folder') }}</h2>
          <p class="path-value">{{ store.report?.root }}</p>
        </div>
        <AppButton :disabled="store.busy" @click="router.push('/setup')">{{
          t('changeFolder')
        }}</AppButton>
      </div>
    </section>
    <details class="technical-details save-source-settings">
      <summary>{{ t('sgSourceSettings') }}</summary>
      <p>{{ t('sgSourceSettingsHint') }}</p>
      <p class="path-value">{{ source }}</p>
      <p v-if="sourceError" role="alert">{{ t(sourceError) }}</p>
      <div class="source-actions">
        <AppButton variant="quiet" :loading="sourceBusy" @click="changeSource(false)">{{
          t('sgChooseSource')
        }}</AppButton
        ><AppButton variant="quiet" :disabled="sourceBusy" @click="changeSource(true)">{{
          t('sgResetSource')
        }}</AppButton>
      </div>
    </details>
    <section class="section">
      <h2>{{ t('about') }}</h2>
      <p>{{ t('aboutDescription') }}</p>
    </section>
  </div>
</template>
<style scoped>
.save-source-settings {
  color: var(--color-text-secondary);
}
.save-source-settings p {
  margin-block: 1rem;
  overflow-wrap: anywhere;
}
.source-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}
</style>
