import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { ScanReport } from '../../src/types/installation'

const api = vi.hoisted(() => ({
  scan: vi.fn(),
  loadPreferences: vi.fn(),
  saveLanguage: vi.fn(),
  synchronizeGameLanguage: vi.fn(),
  initializeGameSettings: vi.fn(),
  reconcileGraphicsMods: vi.fn(),
  pickDirectory: vi.fn(),
  onProgress: vi.fn(async () => vi.fn()),
  cancel: vi.fn(),
}))
vi.mock('../../src/services/desktop', () => ({
  desktop: api,
  desktopAvailable: true,
  errorCode: (error: { code?: string }) => error.code ?? 'unknownError',
}))
import { useInstallation } from '../../src/stores/installation'
import { locale, selectedLanguage } from '../../src/composables/i18n'

const report = {
  schemaVersion: 1,
  recognized: true,
  root: 'C:\\Synthetic fixture',
  scannedAt: 0,
  fileCount: 0,
  totalBytes: 0,
  pkzCount: 1,
  files: [],
  binaries: [],
  checks: [],
  verification: 'UNVERIFIED',
  repairStatus: 'UNKNOWN',
} satisfies ScanReport

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  selectedLanguage.value = 'auto'
  api.initializeGameSettings.mockResolvedValue(false)
  api.reconcileGraphicsMods.mockResolvedValue(null)
  api.synchronizeGameLanguage.mockImplementation(async (language: string) => ({
    requested: language,
    applied: language,
    changed: false,
  }))
})

describe('installation workflow', () => {
  it('synchronizes the resolved automatic language after recognizing a saved installation', async () => {
    api.loadPreferences.mockResolvedValue({ language: 'auto', installationPath: report.root })
    api.scan.mockResolvedValue(report)
    const store = useInstallation()
    await store.initialize()
    expect(api.synchronizeGameLanguage).toHaveBeenCalledExactlyOnceWith(locale.value)
    expect(store.languageState).toBe('synced')
    expect(api.initializeGameSettings).toHaveBeenCalledOnce()
    expect(api.initializeGameSettings.mock.invocationCallOrder[0]!).toBeGreaterThan(
      api.scan.mock.invocationCallOrder[0]!,
    )
    expect(api.synchronizeGameLanguage.mock.invocationCallOrder[0]).toBeGreaterThan(
      api.initializeGameSettings.mock.invocationCallOrder[0]!,
    )
  })
  it('uses the verified report from automatic mods and leaves diagnostics read-only', async () => {
    api.loadPreferences.mockResolvedValue({ language: 'fr', installationPath: report.root })
    api.scan.mockResolvedValue(report)
    api.reconcileGraphicsMods.mockResolvedValue({ ...report, scannedAt: 42 })
    const store = useInstallation()
    await store.initialize()
    expect(store.report?.scannedAt).toBe(42)
    expect(api.reconcileGraphicsMods).toHaveBeenCalledExactlyOnceWith(true)
    expect(api.reconcileGraphicsMods.mock.invocationCallOrder[0]!).toBeGreaterThan(
      api.scan.mock.invocationCallOrder[0]!,
    )
    await store.scanWhenIdle(report.root)
    expect(api.reconcileGraphicsMods).toHaveBeenCalledOnce()
    expect(api.initializeGameSettings).toHaveBeenCalledOnce()
  })
  it('shows a registry setup failure without continuing to language or mods', async () => {
    api.loadPreferences.mockResolvedValue({ language: 'fr', installationPath: report.root })
    api.scan.mockResolvedValue(report)
    api.initializeGameSettings.mockRejectedValueOnce({ code: 'recoveryRequired' })
    const store = useInstallation()
    await store.initialize()
    expect(store.error).toBe('recoveryRequired')
    expect(api.synchronizeGameLanguage).not.toHaveBeenCalled()
    expect(api.reconcileGraphicsMods).not.toHaveBeenCalled()
  })
  it('refreshes diagnostics after creating settings when no default mod is installed', async () => {
    api.loadPreferences.mockResolvedValue({ language: 'fr', installationPath: report.root })
    api.scan.mockResolvedValueOnce(report).mockResolvedValueOnce({ ...report, scannedAt: 43 })
    api.initializeGameSettings.mockResolvedValueOnce(true)
    const store = useInstallation()
    await store.initialize()
    expect(api.scan).toHaveBeenCalledTimes(2)
    expect(store.report?.scannedAt).toBe(43)
  })
  it('keeps the installation available when automatic mods cannot be applied', async () => {
    api.loadPreferences.mockResolvedValue({ language: 'fr', installationPath: report.root })
    api.scan.mockResolvedValue(report)
    api.reconcileGraphicsMods.mockRejectedValueOnce({ code: 'modUnmanagedFiles' })
    const store = useInstallation()
    await store.initialize()
    expect(store.report?.recognized).toBe(true)
    expect(store.graphicsModsError).toBe('modUnmanagedFiles')
    expect(store.error).toBeNull()
  })
  it('saves a shared choice immediately, defers a busy game, and reports an English fallback', async () => {
    api.saveLanguage.mockResolvedValue(undefined)
    api.synchronizeGameLanguage.mockRejectedValueOnce({ code: 'gameAlreadyRunning' })
    const store = useInstallation()
    store.report = report
    await store.setLanguage('fr')
    expect(selectedLanguage.value).toBe('fr')
    expect(api.synchronizeGameLanguage).toHaveBeenCalledWith('fr')
    expect(store.languageState).toBe('pending')
    api.synchronizeGameLanguage.mockResolvedValueOnce({
      requested: 'fr',
      applied: 'en',
      changed: true,
    })
    await store.synchronizeLanguage()
    expect(store.languageState).toBe('fallback')
    expect(selectedLanguage.value).toBe('fr')
  })
  it('keeps a synchronization failure visible without undoing a saved interface preference', async () => {
    api.saveLanguage.mockResolvedValue(undefined)
    api.synchronizeGameLanguage.mockRejectedValueOnce({ code: 'writeFailed' })
    const store = useInstallation()
    store.report = report
    await store.setLanguage('it')
    expect(selectedLanguage.value).toBe('it')
    expect(store.languageState).toBe('error')
    expect(store.languageError).toBe('writeFailed')
    expect(store.languageBusy).toBe(false)
  })
  it('retries a temporary startup lock but does not retry a file-access error', async () => {
    api.loadPreferences.mockResolvedValue({ language: 'auto', installationPath: report.root })
    api.scan.mockRejectedValueOnce({ code: 'installationBusy' }).mockResolvedValueOnce(report)
    const store = useInstallation()
    await store.initialize()
    expect(api.scan).toHaveBeenCalledTimes(2)
    expect(store.report?.recognized).toBe(true)
    expect(store.error).toBeNull()
    api.scan.mockClear().mockRejectedValue({ code: 'fileUnreadable' })
    await store.initialize()
    expect(api.scan).toHaveBeenCalledTimes(1)
    expect(store.error).toBe('fileUnreadable')
  })
  it('revalidates a stored directory instead of trusting preferences', async () => {
    api.loadPreferences.mockResolvedValue({ language: 'auto', installationPath: report.root })
    api.scan.mockResolvedValue({ ...report, recognized: false, verification: 'INVALID' })
    const store = useInstallation()
    await store.initialize()
    expect(api.scan).toHaveBeenCalledWith(report.root)
    expect(store.report?.recognized).toBe(false)
    expect(api.synchronizeGameLanguage).not.toHaveBeenCalled()
    expect(api.reconcileGraphicsMods).not.toHaveBeenCalled()
    expect(store.initializing).toBe(false)
  })
  it('does not start another scan while the first is pending', async () => {
    let finish!: (value: ScanReport) => void
    api.scan.mockImplementation(
      () =>
        new Promise<ScanReport>((resolve) => {
          finish = resolve
        }),
    )
    const store = useInstallation()
    const pending = store.scan(report.root)
    await vi.waitFor(() => expect(api.scan).toHaveBeenCalledOnce())
    expect(await store.scan(report.root)).toBe(false)
    finish(report)
    expect(await pending).toBe(true)
    expect(store.busy).toBe(false)
    expect(store.report?.verification).toBe('UNVERIFIED')
  })
  it('preserves the confirmed language if persistence fails', async () => {
    api.saveLanguage.mockRejectedValue({ code: 'preferencesUnavailable' })
    const store = useInstallation()
    await store.setLanguage('fr')
    expect(selectedLanguage.value).toBe('auto')
    expect(store.error).toBe('preferencesUnavailable')
  })
  it('preserves the selected path when the native picker is cancelled', async () => {
    api.pickDirectory.mockResolvedValue(null)
    const store = useInstallation()
    store.path = report.root
    await store.pick()
    expect(store.path).toBe(report.root)
  })
})
