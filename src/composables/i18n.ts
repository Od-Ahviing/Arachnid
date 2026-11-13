import { computed, ref, watchEffect } from 'vue'
import { en } from '../locales/en'
import { fr } from '../locales/fr'
import { de } from '../locales/de'
import { it } from '../locales/it'
import { es } from '../locales/es'
import { supportedLanguages, type Language, type SupportedLanguage } from '../types/installation'
import type { MessageDictionary } from '../locales/en'

export const messages = { en, fr, de, it, es } satisfies Record<
  SupportedLanguage,
  MessageDictionary
>
export type MessageKey = keyof typeof en

export function resolveLocale(languages: readonly string[]): SupportedLanguage {
  for (const language of languages) {
    const base = language.toLowerCase().split(/[-_]/)[0]
    if (supportedLanguages.includes(base as SupportedLanguage)) return base as SupportedLanguage
  }
  return 'en'
}

export const selectedLanguage = ref<Language>('auto')
export const locale = computed(() =>
  selectedLanguage.value === 'auto'
    ? resolveLocale(globalThis.navigator?.languages ?? ['en'])
    : selectedLanguage.value,
)

export function t(key: string): string {
  const dictionary: Record<string, string> = messages[locale.value]
  return dictionary[key] ?? en[key as MessageKey] ?? dictionary.unknownError
}

export function initializeDocumentLanguage() {
  watchEffect(() => {
    document.documentElement.lang = locale.value
  })
}
