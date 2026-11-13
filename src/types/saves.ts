export interface SaveProfile {
  schemaVersion: 1
  id: string
  source: string
  enabled: boolean
  automatic?: boolean
  manualSource?: boolean
  keep: number
  quotaBytes: number
}

export interface Snapshot {
  id: string
  profileId: string
  createdAt: number
  reason: string
  pinned: boolean
  status: 'copyVerified'
  files: { path: string; bytes: number; sha256: string }[]
}

export interface RestorePreview {
  digest: string
  phase:
    | 'prepared'
    | 'applying'
    | 'committed'
    | 'rollingBack'
    | 'rolledBack'
    | 'cancelled'
    | 'recoveryRequired'
  files: {
    path: string
    beforeBytes: number | null
    afterBytes: number | null
    beforeHash: string | null
    afterHash: string | null
  }[]
}

export interface SaveStatus {
  backupDirectory: string
  profile: SaveProfile | null
  snapshots: Snapshot[]
  operation: RestorePreview | null
  error: string | null
  busy: boolean
}
