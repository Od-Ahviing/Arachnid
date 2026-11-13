import { defineStore } from 'pinia'
import { ref } from 'vue'
import { desktop, desktopAvailable, errorCode } from '../services/desktop'
import { locale, selectedLanguage } from '../composables/i18n'
import type { Language, ScanProgress, ScanReport } from '../types/installation'

export const useInstallation = defineStore('installation', () => {
  const report = ref<ScanReport | null>(null)
  const busy = ref(false)
  const initializing = ref(true)
  const error = ref<string | null>(null)
  const path = ref('')
  const languageBusy = ref(false)
  const languageState = ref<'synced' | 'fallback' | 'pending' | 'error' | null>(null)
  const languageError = ref<string | null>(null)
  const progress = ref<ScanProgress>({ stage: 'inventory', filesScanned: 0 })

  async function scan(directory = path.value): Promise<boolean> {
    if (busy.value) return false
    busy.value = true
    error.value = null
    progress.value = { stage: 'inventory', filesScanned: 0 }
    let unlisten: (() => void) | undefined
    try {
      unlisten = await desktop.onProgress((value) => {
        progress.value = value
      })
      const result = await desktop.scan(directory)
      report.value = result
      path.value = result.root
      return result.recognized
    } catch (failure) {
      error.value = errorCode(failure)
      return false
    } finally {
      unlisten?.()
      busy.value = false
    }
  }

  async function pick(): Promise<void> {
    error.value = null
    try {
      const directory = await desktop.pickDirectory()
      if (directory !== null) path.value = directory
    } catch (failure) {
      error.value = errorCode(failure)
    }
  }

  async function scanWhenIdle(directory: string): Promise<boolean> {
    for (let attempt = 0; attempt < 4; attempt++) {
      const scanned = await scan(directory)
      if (scanned || (error.value !== 'installationBusy' && error.value !== 'scanBusy')) {
        return scanned
      }
      if (attempt < 3) await new Promise((resolve) => setTimeout(resolve, 250))
    }
    return false
  }

  async function initialize(): Promise<void> {
    initializing.value = true
    if (!desktopAvailable) {
      initializing.value = false
      return
    }
    try {
      const preferences = await desktop.loadPreferences()
      selectedLanguage.value = preferences.language
      if (preferences.installationPath) {
        path.value = preferences.installationPath
        await scanWhenIdle(path.value)
        if (!error.value && report.value?.recognized) await synchronizeLanguage()
      }
    } catch (failure) {
      error.value = errorCode(failure)
    } finally {
      initializing.value = false
    }
  }

  async function setLanguage(language: Language): Promise<void> {
    if (languageBusy.value) return
    languageBusy.value = true
    error.value = null
    try {
      await desktop.saveLanguage(language)
      selectedLanguage.value = language
      await synchronizeLanguage()
    } catch (failure) {
      error.value = errorCode(failure)
    } finally {
      languageBusy.value = false
    }
  }

  async function synchronizeLanguage(): Promise<boolean> {
    if (!report.value?.recognized) return true
    languageBusy.value = true
    languageError.value = null
    languageState.value = 'pending'
    try {
      const result = await desktop.synchronizeGameLanguage(locale.value)
      languageState.value = result.applied === result.requested ? 'synced' : 'fallback'
      return true
    } catch (failure) {
      const code = errorCode(failure)
      languageError.value = code
      languageState.value = ['gameAlreadyRunning', 'installationBusy', 'scanBusy'].includes(code)
        ? 'pending'
        : 'error'
      return false
    } finally {
      languageBusy.value = false
    }
  }

  async function cancel(): Promise<void> {
    try {
      await desktop.cancel()
    } catch (failure) {
      error.value = errorCode(failure)
    }
  }

  return {
    report,
    busy,
    initializing,
    error,
    path,
    progress,
    scan,
    scanWhenIdle,
    pick,
    initialize,
    setLanguage,
    synchronizeLanguage,
    languageBusy,
    languageState,
    languageError,
    cancel,
  }
})
