<script setup lang="ts">
import { Languages } from '@lucide/vue'
import { useInstallation } from '../../stores/installation'
import { locale, selectedLanguage, t } from '../../composables/i18n'
import { ui } from '../../composables/interfaceMessages'
import type { Language } from '../../types/installation'
import AppButton from '../ui/AppButton.vue'
import BusyState from '../ui/BusyState.vue'
const store = useInstallation()
const languages = [
  { code: 'fr', name: 'Français' },
  { code: 'en', name: 'English' },
  { code: 'de', name: 'Deutsch' },
  { code: 'it', name: 'Italiano' },
  { code: 'es', name: 'Español' },
] as const
function change(event: Event) {
  void store.setLanguage((event.target as HTMLSelectElement).value as Language)
}
</script>
<template>
  <section class="game-language-panel" :aria-busy="store.languageBusy">
    <div class="settings-row">
      <div>
        <label for="shared-language"
          ><Languages :size="17" aria-hidden="true" />{{ ui('sharedLanguage') }}</label
        >
        <p>{{ ui('sharedLanguageHint') }}</p>
      </div>
      <select
        id="shared-language"
        :value="selectedLanguage"
        :disabled="store.languageBusy || store.busy"
        @change="change"
      >
        <option value="auto">
          {{ t('automatic') }} · {{ languages.find((language) => language.code === locale)?.name }}
        </option>
        <option v-for="language in languages" :key="language.code" :value="language.code">
          {{ language.name }}
        </option>
      </select>
    </div>
    <BusyState v-if="store.languageBusy" :label="ui('languageSyncing')" />
    <p v-else-if="store.languageState === 'synced'" class="language-status" role="status">
      {{ ui('languageSynced') }}
    </p>
    <div
      v-else-if="store.languageState"
      class="language-status"
      :role="store.languageState === 'error' ? 'alert' : 'status'"
    >
      <p>
        {{
          ui(
            store.languageState === 'fallback'
              ? 'languageFallback'
              : store.languageState === 'pending'
                ? 'languageDeferred'
                : 'languageSyncFailed',
          )
        }}
      </p>
      <p v-if="store.languageState === 'error' && store.languageError">
        {{ t(store.languageError) }}
      </p>
      <AppButton
        v-if="store.languageState !== 'fallback'"
        variant="quiet"
        :disabled="store.busy"
        @click="store.synchronizeLanguage"
        >{{ t('retry') }}</AppButton
      >
    </div>
  </section>
</template>
<style scoped>
.game-language-panel {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-panel);
  background: var(--color-sidebar);
  padding: 0.5rem 1.5rem 1rem;
  margin-block: 1.5rem;
}
label {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.settings-row {
  border: 0;
}
.language-status {
  font-size: var(--text-caption);
  color: var(--color-text-secondary);
}
</style>
