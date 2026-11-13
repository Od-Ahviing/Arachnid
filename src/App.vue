<script setup lang="ts">
import { nextTick, onMounted, watch, watchEffect } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useInstallation } from './stores/installation'
import { initializeDocumentLanguage, t } from './composables/i18n'
import { ui } from './composables/interfaceMessages'
import { desktopAvailable } from './services/desktop'
import AppSidebar from './components/layout/AppSidebar.vue'
import StartupScreen from './components/layout/StartupScreen.vue'
import BusyState from './components/ui/BusyState.vue'
import AppButton from './components/ui/AppButton.vue'
const store = useInstallation()
const route = useRoute()
const router = useRouter()
initializeDocumentLanguage()
const routeLabels: Record<string, string> = {
  '/setup': 'welcome',
  '/home': 'home',
  '/graphics': 'graphics',
  '/mods': 'mods',
  '/saves': 'saves',
  '/repair': 'repair',
  '/settings': 'settings',
}
watchEffect(() => {
  document.title = `${t(routeLabels[route.path] ?? 'home')} · Arachnid`
})
watch(
  [() => route.path, () => store.initializing],
  async ([, initializing]) => {
    if (initializing) return
    await nextTick()
    document.querySelector<HTMLElement>('main h1')?.focus({ preventScroll: true })
  },
  { flush: 'post' },
)
function focusMain() {
  document.getElementById('main')?.focus()
}
async function initialize() {
  await store.initialize()
  if (store.report?.recognized && route.path === '/setup') await router.replace('/home')
}
function chooseAnotherFolder() {
  store.error = null
  store.path = ''
  void router.replace('/setup')
}
onMounted(initialize)
watch(
  () => store.report?.recognized,
  (recognized) => {
    if (recognized === false) void router.replace('/setup')
  },
)
</script>

<template>
  <a href="#main" class="skip-link" @click.prevent="focusMain">{{ t('skip') }}</a>
  <StartupScreen v-if="store.initializing" />
  <main v-else-if="store.path && !store.report && store.error" id="main" tabindex="-1">
    <section class="setup-panel startup-retry">
      <h1 tabindex="-1">{{ t('errorTitle') }}</h1>
      <p class="intro">{{ ui('startupRetry') }}</p>
      <p class="notice notice--error" role="alert">{{ t(store.error) }}</p>
      <p class="path-value">{{ store.path }}</p>
      <div class="plan-actions">
        <AppButton variant="primary" @click="initialize">{{ t('retry') }}</AppButton>
        <AppButton variant="quiet" @click="chooseAnotherFolder">{{ t('changeFolder') }}</AppButton>
      </div>
    </section>
  </main>
  <div v-else class="app-shell" :class="{ 'app-shell--setup': route.path === '/setup' }">
    <AppSidebar v-if="route.path !== '/setup'" />
    <main id="main" tabindex="-1">
      <div v-if="!desktopAvailable" class="notice" role="status">{{ t('desktopOnly') }}</div>
      <div v-if="store.error" class="notice notice--error" role="alert">
        <strong>{{ t('errorTitle') }}</strong>
        <p>{{ t(store.error) }}</p>
      </div>
      <div v-if="store.busy" class="notice" role="status" aria-live="polite">
        <BusyState :label="t('scanning')" />
        <p>
          {{ t(store.progress.stage) }} · {{ t('filesRead') }} : {{ store.progress.filesScanned }}
        </p>
        <AppButton @click="store.cancel">{{ t('cancel') }}</AppButton>
      </div>
      <RouterView />
    </main>
  </div>
</template>
