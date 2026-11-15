<script setup lang="ts">
import { useRouter } from 'vue-router'
import { FolderOpen, ArrowRight } from '@lucide/vue'
import { useInstallation } from '../stores/installation'
import { ui } from '../composables/interfaceMessages'
import { selectedLanguage, t } from '../composables/i18n'
import { desktopAvailable } from '../services/desktop'
import type { Language } from '../types/installation'
import logo from '../assets/brand/arachnid-logo.png'
import AppButton from '../components/ui/AppButton.vue'
import CheckList from '../components/diagnostics/CheckList.vue'
const store = useInstallation()
const router = useRouter()
async function inspect() {
  if (store.busy || store.languageBusy || store.graphicsModsBusy) return
  if (await store.scanWhenIdle(store.path)) {
    await store.configureGame()
    await router.push('/home')
  }
}
async function chooseFolder() {
  if (await store.pick()) await inspect()
}
function changeLanguage(event: Event) {
  void store.setLanguage((event.target as HTMLSelectElement).value as Language)
}
</script>

<template>
  <section class="setup-panel">
    <RouterLink v-if="store.report?.recognized" to="/home" class="text-link setup-back">{{
      ui('back')
    }}</RouterLink>
    <div class="setup-brand">
      <img :src="logo" alt="" width="64" height="64" /><span>Arachnid</span>
    </div>
    <p class="eyebrow">The Amazing Spider-Man · 2012</p>
    <h1 tabindex="-1">{{ t('welcome') }}</h1>
    <p class="intro">{{ t('setupDescription') }}</p>
    <form class="setup-form" @submit.prevent="inspect">
      <label for="game-folder">{{ t('directory') }}</label>
      <p id="folder-help" class="field-hint">{{ t('directoryHint') }}</p>
      <div class="folder-field">
        <input
          id="game-folder"
          v-model="store.path"
          type="text"
          aria-describedby="folder-help"
          :disabled="store.busy || !desktopAvailable"
          autocomplete="off"
          spellcheck="false"
        />
        <AppButton
          :disabled="
            store.busy || store.graphicsModsBusy || store.languageBusy || !desktopAvailable
          "
          @click="chooseFolder"
          ><FolderOpen :size="17" aria-hidden="true" />{{ t('browse') }}</AppButton
        >
      </div>
      <AppButton
        type="submit"
        variant="primary"
        :loading="store.busy || store.graphicsModsBusy"
        :disabled="
          !store.path.trim() || !desktopAvailable || store.languageBusy || store.graphicsModsBusy
        "
        >{{ t('inspect') }}<ArrowRight :size="17" aria-hidden="true"
      /></AppButton>
    </form>
    <div v-if="store.report && !store.report.recognized" class="setup-results">
      <h2>{{ t('invalid') }}</h2>
      <p>{{ t('invalidHint') }}</p>
      <CheckList :checks="store.report.checks" />
    </div>
    <p class="ownership">{{ t('ownGame') }}</p>
    <div class="setup-language">
      <label for="setup-language">{{ t('language') }}</label
      ><select
        id="setup-language"
        :value="selectedLanguage"
        :disabled="!desktopAvailable || store.languageBusy"
        @change="changeLanguage"
      >
        <option value="auto">{{ t('automatic') }}</option>
        <option value="en">English</option>
        <option value="fr">Français</option>
        <option value="de">Deutsch</option>
        <option value="it">Italiano</option>
        <option value="es">Español</option>
      </select>
    </div>
  </section>
</template>
