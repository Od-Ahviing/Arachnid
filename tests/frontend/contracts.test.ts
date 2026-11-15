import { describe, expect, it } from 'vitest'
import { errorCode, parseReport } from '../../src/services/desktop'
import mainCapability from '../../src-tauri/capabilities/main.json'
import appManifest from '../../src-tauri/build.rs?raw'
import nativeOperations from '../../src-tauri/permissions/m2.toml?raw'

const report = {
  schemaVersion: 1,
  root: 'C:\\Synthetic fixture',
  recognized: true,
  scannedAt: 0,
  fileCount: 0,
  totalBytes: 0,
  pkzCount: 1,
  files: [],
  binaries: [],
  verification: 'UNVERIFIED',
  repairStatus: 'UNKNOWN',
  checks: [{ code: 'runtimeUntested', status: 'unverified', value: null }],
}

describe('desktop contract', () => {
  it('authorizes native settings initialization for the local launcher window', () => {
    expect(appManifest).toContain('"initialize_game_settings"')
    expect(nativeOperations).toMatch(/commands\.allow = \[[^\r\n]*"initialize_game_settings"/)
    expect(mainCapability.permissions).toContain('m2-local-operations')
    expect(mainCapability.windows).toEqual(['main'])
    expect(mainCapability.local).toBe(true)
  })

  it('accepts the current report without promoting its confidence', () => {
    expect(parseReport(report).verification).toBe('UNVERIFIED')
  })
  it('rejects future versions, malformed checks and incomplete responses', () => {
    for (const value of [
      null,
      {},
      { ...report, schemaVersion: 2 },
      { ...report, checks: [{ status: 'green' }] },
    ]) {
      expect(() => parseReport(value)).toThrow()
    }
  })
  it('does not display private exception details', () => {
    expect(errorCode(new Error('C:\\private-path'))).toBe('unknownError')
    expect(errorCode({ code: 'fileUnreadable' })).toBe('fileUnreadable')
  })
})
