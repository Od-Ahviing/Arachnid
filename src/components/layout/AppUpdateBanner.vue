<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { desktop, errorCode } from '../../services/desktop'
import { t } from '../../composables/i18n'
import AppButton from '../ui/AppButton.vue'
import BusyState from '../ui/BusyState.vue'

const props = defineProps<{ visible: boolean; canInstall: boolean }>()

const version = ref<string | null>(null)
const installing = ref(false)
const percent = ref<number | null>(null)
const error = ref<string | null>(null)
const dismissed = ref(false)

onMounted(async () => {
  try {
    version.value = await desktop.checkAppUpdate()
  } catch {
    // Startup checks are optional when the network or release feed is unavailable.
  }
})

async function install() {
  if (!props.canInstall || !version.value || installing.value) return
  installing.value = true
  percent.value = null
  error.value = null
  try {
    await desktop.installAppUpdate(version.value, ({ received, total }) => {
      percent.value = total ? Math.min(100, Math.floor((received / total) * 100)) : null
    })
  } catch (failure) {
    error.value = errorCode(failure)
  } finally {
    installing.value = false
  }
}
</script>

<template>
  <section
    v-if="version && !dismissed && (visible || installing)"
    class="notice update-banner"
    :aria-label="t('appUpdateAvailable')"
  >
    <strong role="status">{{ t('appUpdateAvailable') }} · {{ version }}</strong>
    <template v-if="installing">
      <BusyState :label="t(percent === 100 ? 'appUpdateVerifying' : 'appUpdateDownloading')" />
      <progress
        v-if="percent !== null"
        :aria-label="t('appUpdateDownloading')"
        :value="percent"
        max="100"
      />
    </template>
    <template v-else>
      <p v-if="error" role="alert">{{ t(error) }}</p>
      <div class="plan-actions">
        <AppButton variant="primary" :disabled="!canInstall" @click="install">{{
          t('appUpdateInstall')
        }}</AppButton>
        <AppButton variant="quiet" @click="dismissed = true">{{ t('appUpdateLater') }}</AppButton>
      </div>
    </template>
  </section>
</template>

<style scoped>
.update-banner {
  margin-bottom: var(--space-4, 1rem);
}

progress {
  display: block;
  width: min(100%, 24rem);
}
</style>
