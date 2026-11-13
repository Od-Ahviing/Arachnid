<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Archive, FolderOpen, ShieldCheck, Pause, RotateCcw, Trash2, Pin } from '@lucide/vue'
import { desktop, errorCode } from '../services/desktop'
import { locale, t } from '../composables/i18n'
import AppButton from '../components/ui/AppButton.vue'
import ActionDialog from '../components/ui/ActionDialog.vue'
import type { RestorePreview, SaveStatus, Snapshot } from '../types/saves'
import BusyState from '../components/ui/BusyState.vue'
import { ui } from '../composables/interfaceMessages'
const loading = ref(true)
const status = ref<SaveStatus | null>(null)
const busy = ref(false)
const error = ref('')
const notice = ref('')
const enabled = ref(true)
const keep = ref(20)
const quota = ref(256)
const mode = ref<'restore' | 'review' | 'delete' | 'undo' | null>(null)
const selected = ref<Snapshot | null>(null)
const preview = ref<RestorePreview | null>(null)
const producersStopped = ref(false)
const deletedId = ref<string | null>(null)
let timer: ReturnType<typeof setInterval> | undefined
const snapshots = computed(() => status.value?.snapshots ?? [])
const pending = computed(
  () =>
    status.value?.operation &&
    !['rolledBack', 'cancelled', 'committed'].includes(status.value.operation.phase),
)
const title = computed(() =>
  t(
    mode.value === 'delete'
      ? 'sgDeleteTitle'
      : mode.value === 'review'
        ? 'sgConfirmTitle'
        : mode.value === 'undo'
          ? 'sgUndoTitle'
          : 'sgRestoreTitle',
  ),
)
function date(time: number) {
  return new Date(time * 1000).toLocaleString(locale.value, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}
function size(bytes: number) {
  return new Intl.NumberFormat(locale.value, {
    style: 'unit',
    unit: bytes >= 1024 ? 'kilobyte' : 'byte',
    maximumFractionDigits: 1,
  }).format(bytes >= 1024 ? bytes / 1024 : bytes)
}
function fileCount(count: number) {
  return `${count} ${t(count === 1 ? 'sgFile' : 'sgFiles').toLocaleLowerCase(locale.value)}`
}
function total(snapshot: Snapshot) {
  return snapshot.files.reduce((sum, file) => sum + file.bytes, 0)
}
function reason(snapshot: Snapshot) {
  return t(
    snapshot.reason === 'beforeRestore'
      ? 'sgPrecaution'
      : snapshot.reason === 'manual'
        ? 'sgManual'
        : 'sgAutomatic',
  )
}
function displayPath(path: string) {
  return path.startsWith('\\\\?\\UNC\\')
    ? `\\\\${path.slice(8)}`
    : path.startsWith('\\\\?\\')
      ? path.slice(4)
      : path
}
async function refresh(syncPreferences = false) {
  try {
    status.value = await desktop.saveStatus()
    if (syncPreferences && status.value.profile) {
      enabled.value = status.value.profile.enabled
      keep.value = status.value.profile.keep
      quota.value = status.value.profile.quotaBytes / 1024 / 1024
    }
  } catch (failure) {
    if (errorCode(failure) !== 'installationBusy') error.value = errorCode(failure)
  }
}
async function act(action: () => Promise<unknown>, success = '') {
  if (busy.value) return false
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    await action()
    notice.value = success
    await refresh()
    return true
  } catch (failure) {
    error.value = errorCode(failure)
    await refresh()
    return false
  } finally {
    busy.value = false
  }
}
function open(action: 'restore' | 'delete', snapshot: Snapshot) {
  selected.value = snapshot
  producersStopped.value = false
  preview.value = null
  error.value = ''
  mode.value = action
}
function resume() {
  preview.value = status.value?.operation ?? null
  producersStopped.value = false
  mode.value = 'review'
}
async function close() {
  if (busy.value) return
  if (
    mode.value === 'review' &&
    preview.value?.phase === 'prepared' &&
    !(await act(() => desktop.applySaveRestore(preview.value!.digest, false, true)))
  )
    return
  mode.value = null
  preview.value = null
}
async function prepare() {
  if (!selected.value || !producersStopped.value) return
  if (
    await act(async () => {
      preview.value = await desktop.prepareSaveRestore(selected.value!.id, true)
    })
  )
    mode.value = 'review'
}
async function restore() {
  if (!preview.value || !producersStopped.value) return
  if (await act(() => desktop.applySaveRestore(preview.value!.digest, true, false), 'sgRestored'))
    mode.value = null
}
async function remove() {
  if (!selected.value) return
  const id = selected.value.id
  if (await act(() => desktop.deleteSave(id), 'sgDeleted')) {
    deletedId.value = id
    mode.value = null
  }
}
async function undoRestore() {
  if (!producersStopped.value || !status.value?.operation) return
  if (
    await act(
      () => desktop.applySaveRestore(status.value!.operation!.digest, true, true),
      'saveDone',
    )
  )
    mode.value = null
}
async function undoDelete() {
  if (deletedId.value && (await act(() => desktop.deleteSave(deletedId.value!, true))))
    deletedId.value = null
}
function openUndo() {
  producersStopped.value = false
  mode.value = 'undo'
}
onMounted(async () => {
  await refresh(true)
  loading.value = false
  timer = setInterval(() => {
    if (!busy.value) void refresh()
  }, 3000)
})
onUnmounted(() => clearInterval(timer))
</script>
<template>
  <div class="page save-page">
    <header class="page-heading save-heading">
      <div>
        <p class="eyebrow">SaveGuard</p>
        <h1 tabindex="-1">{{ t('sgTitle') }}</h1>
        <p class="intro">{{ t('sgIntro') }}</p>
      </div>
      <AppButton :disabled="busy" @click="act(desktop.openSaveFolder)"
        ><FolderOpen :size="17" aria-hidden="true" />{{ t('sgOpenFolder') }}</AppButton
      >
    </header>
    <p v-if="(error || status?.error) && !mode" role="alert" class="notice notice--error">
      {{ t(error || status?.error || '') }}
    </p>
    <div v-if="notice" role="status" class="save-notice">
      <span>{{ t(notice) }}</span
      ><AppButton v-if="deletedId" variant="quiet" :disabled="busy" @click="undoDelete">{{
        t('sgUndoDelete')
      }}</AppButton>
    </div>
    <BusyState v-if="loading || busy" :label="ui('loading')" />
    <AppButton v-if="!loading && !status" :loading="busy" @click="act(() => refresh(true))">{{
      t('retry')
    }}</AppButton>
    <section v-if="status" class="protection-panel">
      <component
        :is="status?.profile?.enabled ? ShieldCheck : Pause"
        :size="28"
        aria-hidden="true"
      />
      <div>
        <h2>
          {{ t(!status?.profile ? 'sgMissing' : status.profile.enabled ? 'sgActive' : 'sgPaused') }}
        </h2>
        <p>{{ t(status?.profile ? 'sgAutoHint' : 'sgMissingHint') }}</p>
      </div>
      <AppButton
        v-if="status?.profile"
        :disabled="busy"
        @click="act(desktop.snapshotSaves, 'saveDone')"
        ><Archive :size="16" aria-hidden="true" />{{ t('sgCopyNow') }}</AppButton
      ><AppButton
        v-else
        :disabled="busy"
        @click="act(() => desktop.detectSaveSource()).then(() => refresh(true))"
        >{{ t('scanAgain') }}</AppButton
      >
    </section>
    <div v-if="pending" class="save-notice">
      <span>{{ t('sgPending') }}</span
      ><AppButton :disabled="busy" @click="resume">{{ t('sgResume') }}</AppButton>
    </div>
    <div v-else-if="status?.operation?.phase === 'committed'" class="save-notice">
      <span>{{ t('sgRestored') }}</span
      ><AppButton :disabled="busy" @click="openUndo"
        ><RotateCcw :size="16" aria-hidden="true" />{{ t('sgUndoRestore') }}</AppButton
      >
    </div>
    <section v-if="!loading && status" class="section history-panel">
      <div class="history-heading">
        <h2>{{ t('sgHistory') }}</h2>
        <span>{{ snapshots.length }}</span>
      </div>
      <div v-if="!snapshots.length" class="save-empty">
        <Archive :size="32" aria-hidden="true" />
        <h3>{{ t('sgEmpty') }}</h3>
        <p>{{ t('sgEmptyHint') }}</p>
      </div>
      <article v-for="(snapshot, index) in snapshots" :key="snapshot.id" class="snapshot-row">
        <div class="snapshot-info">
          <h3>
            <time :datetime="new Date(snapshot.createdAt * 1000).toISOString()">{{
              date(snapshot.createdAt)
            }}</time>
          </h3>
          <p>
            {{ reason(snapshot) }} · {{ fileCount(snapshot.files.length) }} ·
            {{ size(total(snapshot))
            }}<span v-if="snapshot.pinned" class="save-badge">{{ t('sgProtected') }}</span
            ><span v-else-if="index === 0" class="save-badge">{{ t('sgLast') }}</span>
          </p>
          <details>
            <summary>{{ t('sgDetails') }}</summary>
            <p>{{ t('saveVerified') }}</p>
            <p v-for="file in snapshot.files" :key="file.path">
              {{ file.path }} · {{ size(file.bytes) }}
            </p>
          </details>
        </div>
        <div class="snapshot-actions">
          <AppButton :disabled="busy || !!pending" @click="open('restore', snapshot)"
            ><RotateCcw :size="16" aria-hidden="true" />{{ t('sgRestore') }}</AppButton
          ><AppButton
            variant="quiet"
            :disabled="busy"
            :aria-label="`${t(snapshot.pinned ? 'saveUnpin' : 'savePin')} · ${date(snapshot.createdAt)}`"
            :title="t(snapshot.pinned ? 'saveUnpin' : 'savePin')"
            :aria-pressed="snapshot.pinned"
            @click="act(() => desktop.pinSave(snapshot.id, !snapshot.pinned))"
            ><Pin :size="16" aria-hidden="true" /></AppButton
          ><AppButton
            variant="quiet"
            :disabled="busy || !!pending || snapshot.pinned || snapshots.length <= 1"
            :aria-label="`${t('sgDelete')} · ${date(snapshot.createdAt)}`"
            :title="
              t(
                snapshot.pinned
                  ? 'savePinned'
                  : snapshots.length <= 1
                    ? 'saveLastCopy'
                    : 'sgDelete',
              )
            "
            @click="open('delete', snapshot)"
            ><Trash2 :size="16" aria-hidden="true"
          /></AppButton>
        </div>
      </article>
    </section>
    <details v-if="status" class="save-settings technical-details">
      <summary>{{ t('sgAdvanced') }}</summary>
      <form
        v-if="status?.profile"
        @submit.prevent="act(() => desktop.configureSaveguard(enabled, keep, quota), 'saveDone')"
      >
        <label class="save-checkbox"
          ><input v-model="enabled" type="checkbox" :disabled="busy" />{{ t('sgPause') }}</label
        >
        <div class="save-fields">
          <label
            >{{ t('saveKeep')
            }}<input
              v-model.number="keep"
              :disabled="busy"
              type="number"
              min="1"
              max="100"
              required /></label
          ><label
            >{{ t('saveQuota')
            }}<input
              v-model.number="quota"
              :disabled="busy"
              type="number"
              min="8"
              max="1024"
              required
          /></label>
        </div>
        <AppButton type="submit" :disabled="busy">{{ t('saveConfigure') }}</AppButton>
      </form>
      <dl class="save-paths">
        <dt>{{ t('sgSource') }}</dt>
        <dd>{{ status?.profile ? displayPath(status.profile.source) : t('sgMissing') }}</dd>
        <dt>{{ t('sgDestination') }}</dt>
        <dd>{{ displayPath(status?.backupDirectory ?? '') }}</dd>
      </dl>
    </details>
    <ActionDialog :open="mode !== null" :title="title" :busy="busy" @close="close">
      <p v-if="error" role="alert" class="notice notice--error">{{ t(error) }}</p>
      <p v-if="selected && mode !== 'undo'" class="dialog-date">
        {{ date(selected.createdAt) }} · {{ fileCount(selected.files.length) }} ·
        {{ size(total(selected)) }}
      </p>
      <p>
        {{
          t(
            mode === 'delete'
              ? 'sgDeleteHint'
              : mode === 'review'
                ? 'sgConfirmHint'
                : mode === 'undo'
                  ? 'sgUndoHint'
                  : 'sgRestoreHint',
          )
        }}
      </p>
      <template v-if="mode === 'review' && preview"
        ><ul class="file-changes">
          <li v-for="file in preview.files" :key="file.path">
            <strong>{{ file.path }}</strong
            ><span
              >{{ file.beforeBytes === null ? t('saveRemoved') : size(file.beforeBytes) }} →
              {{ file.afterBytes === null ? t('saveRemoved') : size(file.afterBytes) }}</span
            >
          </li>
        </ul>
        <details>
          <summary>{{ t('sgDetails') }} · SHA-256</summary>
          <p class="digest">{{ preview.digest }}</p>
          <p v-for="file in preview.files" :key="file.path" class="digest">
            {{ file.path }}<br />{{ file.beforeHash }} → {{ file.afterHash }}
          </p>
        </details></template
      >
      <label v-if="mode !== 'delete'" class="save-checkbox producer-check"
        ><input v-model="producersStopped" type="checkbox" />{{ t('saveProducers') }}</label
      >
      <div class="dialog-actions">
        <AppButton :disabled="busy" @click="close">{{ t('cancel') }}</AppButton
        ><AppButton
          v-if="mode === 'restore'"
          variant="primary"
          :loading="busy"
          :disabled="!producersStopped"
          @click="prepare"
          >{{ t('sgContinue') }}</AppButton
        ><AppButton
          v-else-if="mode === 'delete'"
          variant="primary"
          :loading="busy"
          @click="remove"
          >{{ t('sgDelete') }}</AppButton
        ><AppButton
          v-else-if="mode === 'undo'"
          variant="primary"
          :loading="busy"
          :disabled="!producersStopped"
          @click="undoRestore"
          >{{ t('sgUndoRestore') }}</AppButton
        ><AppButton
          v-else-if="preview?.phase === 'prepared'"
          variant="primary"
          :loading="busy"
          :disabled="!producersStopped"
          @click="restore"
          >{{ t('sgConfirm') }}</AppButton
        ><AppButton
          v-else
          variant="primary"
          :loading="busy"
          :disabled="!producersStopped"
          @click="undoRestore"
          >{{ t('sgUndoRestore') }}</AppButton
        >
      </div>
    </ActionDialog>
  </div>
</template>
<style scoped>
.save-heading,
.protection-panel,
.snapshot-row,
.save-notice,
.history-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.25rem;
}
.save-heading {
  align-items: flex-start;
}
.save-heading > .app-button,
.protection-panel > .app-button {
  flex-shrink: 0;
}
.protection-panel {
  padding: 1.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-panel);
  background: var(--color-surface);
}
.protection-panel > svg {
  flex-shrink: 0;
  color: var(--color-success);
}
.protection-panel > div {
  flex: 1;
}
.protection-panel h2 {
  font-size: var(--text-section);
  margin-bottom: 0.35rem;
}
.protection-panel p,
.snapshot-info > p,
.save-empty p {
  color: var(--color-text-secondary);
}
.save-notice {
  margin-block: 1rem;
  color: var(--color-text-secondary);
}
.history-heading {
  margin-bottom: 0.75rem;
}
.history-heading > span {
  color: var(--color-text-muted);
}
.snapshot-row {
  padding: 1.15rem 0;
  border-bottom: 1px solid var(--color-border);
  align-items: flex-start;
}
.snapshot-info {
  min-width: 0;
}
.snapshot-info h3 {
  font-size: var(--text-body);
  font-weight: 600;
  margin-bottom: 0.3rem;
}
.snapshot-info > p {
  font-size: var(--text-caption);
}
.snapshot-info details {
  margin-top: 0.65rem;
  font-size: var(--text-caption);
  color: var(--color-text-muted);
}
.snapshot-info details p {
  margin-top: 0.5rem;
  overflow-wrap: anywhere;
}
.snapshot-actions {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-shrink: 0;
}
.save-badge {
  display: inline-block;
  border: 1px solid var(--color-border);
  border-radius: 1rem;
  font-size: 0.7rem;
  padding: 0.1rem 0.5rem;
  margin-left: 0.6rem;
}
.save-empty {
  text-align: center;
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-panel);
  padding: 3rem 1rem;
}
.save-empty svg {
  margin: 0 auto 0.75rem;
  color: var(--color-text-muted);
}
.save-empty h3 {
  margin-bottom: 0.5rem;
}
.save-settings {
  margin-top: 2rem;
  color: var(--color-text-secondary);
}
.save-settings form {
  margin-top: 1rem;
}
.save-fields {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  margin-block: 1.25rem;
}
.save-fields label {
  display: grid;
  gap: 0.5rem;
}
.save-fields input {
  max-width: 10rem;
}
.save-paths {
  display: grid;
  gap: 0.5rem;
  margin-top: 1.5rem;
  font-size: var(--text-caption);
}
.save-paths dd {
  overflow-wrap: anywhere;
  margin-bottom: 0.75rem;
}
.save-paths dt {
  font-weight: 600;
}
.save-checkbox {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  font-weight: 400;
}
.save-checkbox input {
  width: auto;
  margin-top: 0.2rem;
  flex-shrink: 0;
}
.producer-check {
  margin-top: 1.5rem;
}
.dialog-date {
  margin-bottom: 1rem;
  font-weight: 600;
}
.dialog-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 1.75rem;
}
.file-changes {
  list-style: none;
  padding: 0;
  margin-block: 1.25rem;
}
.file-changes li {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  justify-content: space-between;
  border-bottom: 1px solid var(--color-border);
  padding-block: 0.65rem;
}
.digest {
  font-family: var(--font-mono);
  font-size: var(--text-technical);
  overflow-wrap: anywhere;
  margin-block: 0.5rem;
}
@media (max-width: 1050px) {
  .save-heading,
  .protection-panel {
    flex-wrap: wrap;
  }
  .save-heading > div {
    flex-basis: 100%;
  }
  .protection-panel > .app-button {
    margin-left: 2.8rem;
  }
}
@media (max-width: 760px) {
  .snapshot-row {
    flex-direction: column;
  }
  .snapshot-actions {
    width: 100%;
  }
  .snapshot-actions > .app-button:first-child {
    margin-right: auto;
  }
}
</style>
