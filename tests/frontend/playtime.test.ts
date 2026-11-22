import { afterEach, describe, expect, it } from 'vitest'
import { selectedLanguage } from '../../src/composables/i18n'
import { formatPlaytime } from '../../src/composables/playtime'
import { interfaceMessages } from '../../src/composables/interfaceMessages'

afterEach(() => {
  selectedLanguage.value = 'auto'
})

describe('play time', () => {
  it('formats zero, minutes, hours and large totals without a date rollover', () => {
    selectedLanguage.value = 'en'
    expect(formatPlaytime(0)).toBe('Less than a minute')
    expect(formatPlaytime(59)).toBe('Less than a minute')
    expect(formatPlaytime(60)).toBe('1 min')
    expect(formatPlaytime(3661)).toBe('1 hr 1 min')
    expect(formatPlaytime(360000)).toBe('100 hr 0 min')
  })

  it('provides labels and localized durations in all five languages', () => {
    for (const language of ['en', 'fr', 'de', 'it', 'es'] as const) {
      selectedLanguage.value = language
      const messages = interfaceMessages[language]
      expect(formatPlaytime(0)).toBe(messages.playtimeLessMinute)
      expect(formatPlaytime(3660)).toContain('1')
      for (const key of ['playtime', 'playtimeUnavailable'] as const) {
        expect(messages[key].length).toBeGreaterThan(0)
      }
    }
  })
})
