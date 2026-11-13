import { Channel, invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Language, Preferences, ScanProgress, ScanReport } from '../types/installation'
import { supportedLanguages } from '../types/installation'
import type {
  LaunchStatus,
  SettingsOperation,
  SettingsPlan,
  SettingsReport,
} from '../types/operations'
import type { SaveStatus, RestorePreview } from '../types/saves'
import type { ModStatus, PackPreview, SkinPreview, SkinStatus } from '../types/mods'

export const desktopAvailable = isTauri()

type AppUpdateProgress = { received: number; total: number | null }

export function errorCode(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string'
  )
    return error.code
  return 'unknownError'
}

export function parseReport(value: unknown): ScanReport {
  if (typeof value !== 'object' || value === null) throw { code: 'invalidResponse' }
  const report = value as Partial<ScanReport>
  if (
    report.schemaVersion !== 1 ||
    typeof report.root !== 'string' ||
    typeof report.recognized !== 'boolean' ||
    typeof report.scannedAt !== 'number' ||
    typeof report.totalBytes !== 'number' ||
    typeof report.fileCount !== 'number' ||
    typeof report.pkzCount !== 'number' ||
    !Array.isArray(report.files) ||
    !Array.isArray(report.binaries) ||
    !Array.isArray(report.checks) ||
    !['INVALID', 'UNVERIFIED', 'VERIFIED'].includes(report.verification ?? '') ||
    !['UNKNOWN', 'NOT_REQUIRED', 'REPAIR_REQUIRED'].includes(report.repairStatus ?? '') ||
    report.checks.some(
      (check) =>
        !check ||
        typeof check.code !== 'string' ||
        !['passed', 'warning', 'error', 'unverified', 'skipped'].includes(check.status),
    )
  ) {
    throw { code: 'invalidResponse' }
  }
  return report as ScanReport
}

export const desktop = {
  checkAppUpdate: () => invoke<string | null>('check_app_update'),
  installAppUpdate: (version: string, onProgress: (progress: AppUpdateProgress) => void) => {
    const progress = new Channel<AppUpdateProgress>()
    progress.onmessage = onProgress
    return invoke<void>('install_app_update', { version, progress })
  },
  skinStatus: () => invoke<SkinStatus>('skin_status'),
  prepareSkin: (selected: string | null) => invoke<SkinPreview>('prepare_skin', { selected }),
  applySkin: (digest: string, recover = false) => invoke<void>('apply_skin', { digest, recover }),
  modStatus: () => invoke<ModStatus>('mod_status'),
  applyDefaultMods: async () => {
    const report = await invoke<unknown>('apply_default_mods')
    return report === null ? null : parseReport(report)
  },
  preparePack: (
    operation: 'enable' | 'disable' | 'sync',
    components?: import('../types/mods').ModComponents,
  ) => invoke<PackPreview>('prepare_pack', { operation, components }),
  applyPack: async (digest: string, recover = false) => {
    const report = await invoke<unknown>('apply_pack', { digest, recover })
    return report === null ? null : parseReport(report)
  },
  selectMod: (id: string, enabled: boolean) => invoke<ModStatus>('select_mod', { id, enabled }),
  saveStatus: () => invoke<SaveStatus>('save_status'),
  chooseSaveSource: () => invoke<boolean>('choose_save_source'),
  detectSaveSource: (reset = false) => invoke<void>('detect_save_source', { reset }),
  openSaveFolder: () => invoke<void>('open_save_folder'),
  deleteSave: (id: string, undo = false) => invoke<void>('delete_save', { id, undo }),
  configureSaveguard: (enabled: boolean, keep: number, quotaMib: number) =>
    invoke<void>('configure_saveguard', { enabled, keep, quotaMib }),
  snapshotSaves: () => invoke<void>('snapshot_saves'),
  pinSave: (id: string, pinned: boolean) => invoke<void>('pin_save', { id, pinned }),
  prepareSaveRestore: (id: string, producersStopped: boolean) =>
    invoke<RestorePreview>('prepare_save_restore', { id, producersStopped }),
  applySaveRestore: (digest: string, producersStopped: boolean, undo: boolean) =>
    invoke<void>('apply_save_restore', { digest, producersStopped, undo }),
  launchStatus: () => invoke<LaunchStatus>('launch_status'),
  initializeGameSettings: () => invoke<boolean>('initialize_game_settings'),
  startGame: (language: import('../types/installation').SupportedLanguage) =>
    invoke<LaunchStatus>('start_game', { language }),
  synchronizeGameLanguage: (language: import('../types/installation').SupportedLanguage) =>
    invoke<{ requested: string; applied: string; changed: boolean }>('synchronize_game_language', {
      language,
    }),
  nativeSettings: () => invoke<SettingsReport>('native_settings'),
  prepareSettings: (
    changes: { name: string; value: number }[],
    profile?: import('../types/operations').GraphicsProfile,
  ) => invoke<SettingsPlan>('prepare_settings', { changes, ...(profile ? { profile } : {}) }),
  prepareProfile: (profile: import('../types/operations').GraphicsProfile) =>
    invoke<SettingsPlan>('prepare_settings', { changes: [], profile }),
  applySettings: (digest: string) => invoke<void>('apply_settings', { digest }),
  restoreSettings: (digest: string) => invoke<void>('restore_settings', { digest }),
  settingsOperation: () => invoke<SettingsOperation | null>('settings_operation'),
  async loadPreferences(): Promise<Preferences> {
    const value = await invoke<Preferences>('load_preferences')
    if (value.schemaVersion !== 1 || !['auto', ...supportedLanguages].includes(value.language))
      throw { code: 'invalidResponse' }
    return value
  },
  pickDirectory: () => invoke<string | null>('pick_game_directory'),
  scan: async (path: string) => parseReport(await invoke<unknown>('scan_installation', { path })),
  cancel: () => invoke<void>('cancel_scan'),
  saveLanguage: (language: Language) => invoke<void>('save_language', { language }),
  exportReport: () => invoke<boolean>('export_report'),
  onProgress: (callback: (progress: ScanProgress) => void) =>
    listen<ScanProgress>('scan-progress', (event) => callback(event.payload)),
}
