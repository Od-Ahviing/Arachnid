export interface LaunchStatus {
  playtimeSeconds: number | null
  decision: { ready: boolean; reasons: string[] }
  session: {
    state: 'idle' | 'running' | 'exited' | 'unknown'
    pid: number | null
    exitCode: number | null
  }
  recoveryPending: boolean
  graphicsCrashAdvice: 'reshade' | 'graphicsMods' | null
}

export interface NativeSetting {
  name: string
  section: string
  value: number | null
  valueType: number | null
  editable: boolean
}

export type GraphicsProfile = 'veryLow' | 'low' | 'medium' | 'high' | 'amazing'

export interface SettingsReport {
  settings: NativeSetting[]
  buildSupported: boolean
  runtimeQualified: boolean
  profiles: {
    name: GraphicsProfile
    available: boolean
    settings: { name: string; value: number }[]
  }[]
  currentProfile: 'custom' | GraphicsProfile
  gameLanguages: {
    code: string
    nativeName: string
    value: number
    available: boolean
    resource: string
  }[]
  capabilities: {
    name: string
    kind: 'native' | 'external' | 'derived'
    state: 'readOnly' | 'experimental' | 'localUserTest' | 'unavailable'
    reason: string
  }[]
}

export interface OperationPlan {
  schemaVersion: number
  installation: string
  actions: {
    target: { kind: 'registry'; section: string; name: string }
    before: { valueType: number; bytes: number[] } | null
    after: { valueType: number; bytes: number[] } | null
  }[]
}

export interface SettingsPlan {
  digest: string
  plan: OperationPlan
}

export interface SettingsOperation extends SettingsPlan {
  phase:
    | 'prepared'
    | 'applying'
    | 'committed'
    | 'rollingBack'
    | 'rolledBack'
    | 'cancelled'
    | 'recoveryRequired'
}
