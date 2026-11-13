<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { desktop, errorCode } from '../services/desktop'
import { useInstallation } from '../stores/installation'
import type { ModComponents, ModStatus, SkinStatus } from '../types/mods'
import type { ScanReport } from '../types/installation'
import { modsUiText as m } from '../composables/modsUiMessages'
import { packError } from '../composables/packMessages'
import { ui } from '../composables/interfaceMessages'
import { useDraftNavigation } from '../composables/draftNavigation'
import AppButton from '../components/ui/AppButton.vue'
import ModCatalog from '../components/mods/ModCatalog.vue'
import SkinSelector from '../components/mods/SkinSelector.vue'
import BusyState from '../components/ui/BusyState.vue'
import DraftBar from '../components/ui/DraftBar.vue'
import DraftLeaveDialog from '../components/ui/DraftLeaveDialog.vue'
import ActionDialog from '../components/ui/ActionDialog.vue'

const store = useInstallation()
const tab = ref<'graphics' | 'costumes'>('graphics')
const status = ref<ModStatus | null>(null)
const skins = ref<SkinStatus | null>(null)
const draft = ref<ModComponents>({ fix: false, dxvk: false, reshade: false })
const costume = ref('')
const phase = ref<'loading' | 'applying' | 'verifying' | null>('loading')
const failure = ref<string | null>(null)
const done = ref(false)
const verificationFailed = ref(false)
const recoveryDialog = ref(false)
const busy = computed(() => phase.value !== null || store.busy)
const keys = ['fix', 'dxvk', 'reshade'] as const
const graphicsChanged = computed(
  () =>
    !!status.value &&
    (keys.some((key) => draft.value[key] !== status.value!.components[key]) ||
      status.value.pack.settingsOutOfSync ||
      status.value.pack.requiresUpdate),
)
const costumeChanged = computed(
  () =>
    !!skins.value && ((skins.value.selected ?? '') !== costume.value || skins.value.requiresUpdate),
)
const dirty = computed(() => graphicsChanged.value || costumeChanged.value)
const changeDetails = computed(() => [
  ...keys
    .filter((key) => status.value && draft.value[key] !== status.value.components[key])
    .map((key) => `${m(key)} · ${ui(draft.value[key] ? 'willEnable' : 'willDisable')}`),
  ...(costumeChanged.value
    ? [
        ui('costumes') +
          ' · ' +
          (skins.value?.skins.find((skin) => skin.id === costume.value)?.name ??
            ui('originalTextures')),
      ]
    : []),
])
const recovery = computed(
  () => status.value?.pack.recoveryRequired || skins.value?.recoveryRequired || false,
)
const { leaving, finish } = useDraftNavigation(dirty, busy)
const failureText = computed(() =>
  failure.value ? packError(failure.value) : verificationFailed.value ? ui('verifyFailed') : '',
)

async function refresh() {
  for (let attempt = 0; ; attempt++) {
    try {
      status.value = await desktop.modStatus()
      skins.value = await desktop.skinStatus()
      return
    } catch (error) {
      if (attempt >= 3 || !['installationBusy', 'scanBusy'].includes(errorCode(error))) throw error
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
}
function discard() {
  if (status.value) draft.value = { ...status.value.components }
  costume.value = skins.value?.selected ?? ''
  done.value = false
  if (leaving.value) finish(true)
}
function choose(next: ModComponents) {
  if (busy.value || recovery.value) return
  draft.value = { ...next }
  done.value = false
  failure.value = null
}
async function load() {
  phase.value = 'loading'
  failure.value = null
  try {
    try {
      await refresh()
    } catch (error) {
      const root = store.report?.root
      if (errorCode(error) !== 'noReport' || !root || !(await store.scanWhenIdle(root))) throw error
      await refresh()
    }
    discard()
    if (
      status.value &&
      !status.value.pack.phase &&
      status.value.selection.enabled.includes('performance-pack')
    ) {
      draft.value = { ...status.value.defaultComponents }
    }
  } catch (error) {
    failure.value = errorCode(error)
  } finally {
    phase.value = null
  }
}
async function verify(report?: ScanReport | null) {
  phase.value = 'verifying'
  if (report) store.report = report
  const root = report?.root ?? store.report?.root
  verificationFailed.value =
    !root ||
    !(report ? report.recognized : await store.scanWhenIdle(root)) ||
    !!store.report?.checks.some((check) => check.status === 'error')
  return !verificationFailed.value
}
async function finishVerification(report?: ScanReport | null) {
  if (!(await verify(report))) return
  await refresh()
  if (status.value?.pack.filesChanged || skins.value?.filesChanged) {
    throw { code: 'modOwnedFileChanged' }
  }
  if (dirty.value || recovery.value) return
  done.value = true
  if (leaving.value) finish(true)
}
async function retryVerification() {
  if (busy.value) return
  failure.value = null
  try {
    await finishVerification()
  } catch (error) {
    failure.value = errorCode(error)
    verificationFailed.value = true
  } finally {
    phase.value = null
  }
}
async function apply() {
  if (busy.value) return
  if (verificationFailed.value) {
    await retryVerification()
    return
  }
  if (recovery.value) {
    recoveryDialog.value = true
    return
  }
  if (
    (graphicsChanged.value && status.value?.pack.filesChanged) ||
    (costumeChanged.value && skins.value?.filesChanged)
  ) {
    failure.value = 'modOwnedFileChanged'
    return
  }
  phase.value = 'applying'
  done.value = false
  failure.value = null
  let applied = false
  let report: ScanReport | null | undefined
  try {
    if (graphicsChanged.value) {
      const operation = keys.some((key) => draft.value[key]) ? 'enable' : 'disable'
      const plan = await desktop.preparePack(operation, { ...draft.value })
      if (plan.files.some((file) => file.conflict)) throw { code: 'modOwnedFileChanged' }
      report = await desktop.applyPack(plan.digest, false)
      applied = true
      if (!report && costumeChanged.value) {
        if (!(await verify())) return
        report = store.report
        phase.value = 'applying'
      }
    }
    if (costumeChanged.value) {
      const plan = await desktop.prepareSkin(costume.value || null)
      report = null
      await desktop.applySkin(plan.digest, false)
      applied = true
    }
    await finishVerification(report)
    if (verificationFailed.value) return
    if (dirty.value) throw { code: 'fileChanged' }
  } catch (error) {
    failure.value = errorCode(error)
    if (applied && !(await verify(report))) return
    try {
      await refresh()
    } catch {
      if (applied) verificationFailed.value = true
    }
  } finally {
    phase.value = null
  }
}
async function recover() {
  if (busy.value) return
  phase.value = 'applying'
  failure.value = null
  try {
    let report: ScanReport | null | undefined
    if (status.value?.pack.recoveryRequired && status.value.preview)
      report = await desktop.applyPack(status.value.preview.digest, true)
    if (skins.value?.recoveryRequired && skins.value.preview) {
      if (report === null && !(await verify())) {
        recoveryDialog.value = false
        return
      }
      await desktop.applySkin(skins.value.preview.digest, true)
      report = null
    }
    recoveryDialog.value = false
    if (!(await verify(report))) return
    await refresh()
    if (status.value) draft.value = { ...status.value.components }
    costume.value = skins.value?.selected ?? ''
    if (leaving.value) finish(true)
  } catch (error) {
    failure.value = errorCode(error)
  } finally {
    phase.value = null
  }
}
onMounted(load)
</script>
<template>
  <div class="page mods-page" :aria-busy="busy">
    <header class="page-heading">
      <p class="eyebrow">Mods</p>
      <h1 tabindex="-1">{{ m('title') }}</h1>
      <p class="intro">{{ m('intro') }}</p>
      <p class="offline-note">{{ m('offline') }}</p>
    </header>
    <DraftBar
      :dirty="dirty"
      :details="changeDetails"
      :busy="busy && phase !== 'loading'"
      @apply="apply"
      @discard="discard"
    />
    <BusyState v-if="phase" :label="phase === 'verifying' ? ui('verifying') : m(phase)" />
    <p v-if="failureText" class="notice notice--error" role="alert">{{ failureText }}</p>
    <AppButton v-if="verificationFailed" :loading="busy" @click="retryVerification">{{
      m('retry')
    }}</AppButton>
    <AppButton v-if="(!status || !skins) && phase !== 'loading'" :loading="busy" @click="load">{{
      m('retry')
    }}</AppButton>
    <p v-if="done" class="success-message" role="status">{{ ui('verified') }}</p>
    <div v-if="recovery" class="notice">
      <p>{{ m('recover') }}</p>
      <AppButton :disabled="busy" @click="recoveryDialog = true">{{ m('recover') }}</AppButton>
    </div>
    <div v-if="status || skins" class="section-switcher" :aria-label="m('title')">
      <button type="button" :aria-pressed="tab === 'graphics'" @click="tab = 'graphics'">
        {{ ui('graphicsMods')
        }}<span v-if="graphicsChanged" class="pending-dot" aria-hidden="true" />
      </button>
      <button type="button" :aria-pressed="tab === 'costumes'" @click="tab = 'costumes'">
        {{ ui('costumes') }}<span v-if="costumeChanged" class="pending-dot" aria-hidden="true" />
      </button>
    </div>
    <section v-if="status" v-show="tab === 'graphics'" class="graphics-mods">
      <div class="mods-toolbar">
        <div>
          <h2>{{ ui('graphicsMods') }}</h2>
          <p class="field-hint">{{ ui('selectionHint') }}</p>
        </div>
        <div class="quick-actions">
          <AppButton
            variant="quiet"
            :disabled="busy || recovery"
            @click="choose(status.defaultComponents)"
            >{{ ui('defaults') }}</AppButton
          >
          <AppButton
            variant="quiet"
            :disabled="busy || recovery"
            @click="choose({ fix: true, dxvk: true, reshade: true })"
            >{{ m('all') }}</AppButton
          >
          <AppButton
            variant="quiet"
            :disabled="busy || recovery"
            @click="choose({ fix: false, dxvk: false, reshade: false })"
            >{{ m('none') }}</AppButton
          >
        </div>
      </div>
      <ModCatalog
        :status="status"
        :draft="draft"
        :busy="busy || recovery"
        @change="(key, enabled) => choose({ ...draft, [key]: enabled })"
      />
      <p v-if="status.pack.filesChanged" class="notice notice--error" role="alert">
        {{ m('conflict') }}
      </p>
      <p v-else-if="!status.activationAvailable && !status.pack.active" role="status">
        {{ m('unavailable') }}
      </p>
      <p v-else-if="!graphicsChanged" class="selection-state" role="status">{{ m('ready') }}</p>
    </section>
    <SkinSelector
      v-if="skins"
      v-show="tab === 'costumes'"
      v-model="costume"
      :status="skins"
      :disabled="busy || recovery"
      @update:model-value="done = false"
    />
    <DraftLeaveDialog
      :open="leaving"
      :details="changeDetails"
      :busy="busy"
      :error="failureText"
      @apply="apply"
      @discard="discard"
      @stay="finish(false)"
    />
    <ActionDialog
      :open="recoveryDialog"
      :title="m('recover')"
      :busy="busy"
      @close="recoveryDialog = false"
    >
      <p>{{ m('reviewHint') }}</p>
      <p v-if="failureText" role="alert">{{ failureText }}</p>
      <div class="dialog-actions">
        <AppButton :disabled="busy" @click="recoveryDialog = false">{{ m('cancel') }}</AppButton>
        <AppButton variant="primary" :loading="busy" @click="recover">{{ m('confirm') }}</AppButton>
      </div>
    </ActionDialog>
  </div>
</template>
<style scoped>
.offline-note {
  color: var(--color-text-secondary);
  font-size: var(--text-caption);
  margin-top: 0.5rem;
}
.mods-toolbar,
.quick-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}
.mods-toolbar {
  justify-content: space-between;
  margin-bottom: 1.25rem;
}
.mods-toolbar h2 {
  margin-bottom: 0.35rem;
}
</style>
