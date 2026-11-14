export interface ModRecipe {
  id: string
  name: string
  author: string
  source: string
  license: { name: string; source: string } | null
}

export interface ModComponents {
  fix: boolean
  dxvk: boolean
  reshade: boolean
}

export interface SkinStatus {
  skins: { id: string; name: string; author: string }[]
  selected: string | null
  available: boolean
  filesChanged: boolean
  recoveryRequired: boolean
  requiresUpdate: boolean
  preview: SkinPreview | null
}

export interface SkinPreview {
  digest: string
  selected: string | null
  phase: string
  files: string[]
}

export interface ModStatus {
  components: ModComponents
  defaultComponents: ModComponents
  catalog: {
    groups: { id: string; name: string; members: string[]; defaultEnabled: boolean }[]
    entries: ModRecipe[]
  }
  selection: { schemaVersion: number; enabled: string[] }
  activationAvailable: boolean
  pack: {
    active: boolean
    recoveryRequired: boolean
    settingsOutOfSync: boolean
    filesChanged: boolean
    filesRestored: boolean
    requiresUpdate: boolean
    phase: string | null
  }
  preview: PackPreview | null
}

export interface PackPreview {
  digest: string
  operation: 'enable' | 'disable' | 'sync'
  display: { width: number; height: number; windowed: boolean }
  phase: string
  files: {
    name: string
    before: { sha256: string; bytes: number } | null
    after: { sha256: string; bytes: number } | null
    conflict: boolean
  }[]
}
