export const supportedLanguages = ['fr', 'en', 'de', 'it', 'es'] as const
export type SupportedLanguage = (typeof supportedLanguages)[number]
export type Language = 'auto' | SupportedLanguage
export type CheckStatus = 'passed' | 'warning' | 'error' | 'unverified' | 'skipped'

export interface Preferences {
  schemaVersion: number
  language: Language
  installationPath: string | null
}

export interface Check {
  code: string
  status: CheckStatus
  value: string | null
}

export interface PeInfo {
  name: string
  architecture: string
  sha256: string
  bytes: number
  imports: string[]
  importsInspected: boolean
  largeAddressAware: boolean
}

export interface ScanReport {
  schemaVersion: number
  root: string
  scannedAt: number
  recognized: boolean
  verification: 'INVALID' | 'UNVERIFIED' | 'VERIFIED'
  repairStatus: 'UNKNOWN' | 'NOT_REQUIRED' | 'REPAIR_REQUIRED'
  files: { name: string; bytes: number }[]
  fileCount: number
  totalBytes: number
  pkzCount: number
  binaries: PeInfo[]
  checks: Check[]
}

export interface ScanProgress {
  stage: 'inventory' | 'binaries'
  filesScanned: number
}
