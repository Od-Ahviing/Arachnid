import { describe, expect, it } from 'vitest'
import { messages, resolveLocale, selectedLanguage, t } from '../../src/composables/i18n'

describe('interface language', () => {
  it('resolves supported device variants and defaults to English', () => {
    expect(resolveLocale(['fr-CA'])).toBe('fr')
    expect(resolveLocale(['en-GB'])).toBe('en')
    expect(resolveLocale(['ja-JP'])).toBe('en')
    expect(resolveLocale([])).toBe('en')
    expect(resolveLocale(['de-DE', 'fr-FR'])).toBe('de')
    expect(resolveLocale(['it_IT'])).toBe('it')
    expect(resolveLocale(['es-MX'])).toBe('es')
    expect(resolveLocale(['ja-JP', 'es-ES'])).toBe('es')
  })

  it('provides every message in all five supported interface languages', () => {
    const keys = Object.keys(messages.en).sort()
    expect(Object.keys(messages).sort()).toEqual(['de', 'en', 'es', 'fr', 'it'])
    for (const dictionary of Object.values(messages)) {
      expect(Object.keys(dictionary).sort()).toEqual(keys)
      expect(Object.values(dictionary).every((value) => value.trim().length > 0)).toBe(true)
    }
    for (const [language, play] of [
      ['de', 'Spielen'],
      ['it', 'Gioca'],
      ['es', 'Jugar'],
    ] as const) {
      selectedLanguage.value = language
      expect(t('play')).toBe(play)
      expect(t('languageResourcesMissing')).toBe(messages[language].languageResourcesMissing)
    }
    selectedLanguage.value = 'auto'
  })

  it('uses an explicit selection and keeps meaningful error text', () => {
    selectedLanguage.value = 'fr'
    expect(t('browse')).toBe('Parcourir')
    selectedLanguage.value = 'en'
    expect(t('browse')).toBe('Browse')
    expect(t('missing-code')).toBe(t('unknownError'))
    selectedLanguage.value = 'auto'
  })
})
