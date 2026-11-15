<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { desktop, errorCode } from '../services/desktop'
import { useInstallation } from '../stores/installation'
import { t } from '../composables/i18n'
import { ui } from '../composables/interfaceMessages'
import { useDraftNavigation } from '../composables/draftNavigation'
import type {
  GraphicsProfile,
  SettingsOperation,
  SettingsPlan,
  SettingsReport,
} from '../types/operations'
import AppButton from '../components/ui/AppButton.vue'
import BusyState from '../components/ui/BusyState.vue'
import GraphicsFields from '../components/graphics/GraphicsFields.vue'
import DraftBar from '../components/ui/DraftBar.vue'
import DraftLeaveDialog from '../components/ui/DraftLeaveDialog.vue'

const store = useInstallation()
const report = ref<SettingsReport | null>(null)
const draft = ref<Record<string, number | null>>({})
const baseline = ref<Record<string, number | null>>({})
const selectedProfile = ref<GraphicsProfile | null>(null)
const preview = ref<SettingsPlan | null>(null)
const operation = ref<SettingsOperation | null>(null)
const busy = ref(false)
const phase = ref<'loading' | 'working' | 'verifying'>('loading')
const done = ref(false)
const verificationFailed = ref(false)
const editableSettings = computed(
  () =>
    report.value?.settings.filter((setting) => setting.editable && setting.name !== 'Language') ??
    [],
)
const inspectedSettings = computed(
  () => report.value?.settings.filter((setting) => !setting.editable) ?? [],
)
const settingGroups = computed(
  () =>
    [
      {
        key: 'display',
        fields: editableSettings.value.filter((field) =>
          ['Width', 'Height', 'Fullscreen', 'VSync'].includes(field.name),
        ),
      },
      {
        key: 'quality',
        fields: editableSettings.value.filter((field) =>
          ['MaxAniso', 'AntiAliasing', 'MotionBlur', 'ShadowDetail'].includes(field.name),
        ),
      },
    ] as const,
)
const advancedFields = computed(() =>
  editableSettings.value.filter(
    (field) =>
      ![
        'Width',
        'Height',
        'Fullscreen',
        'VSync',
        'Language',
        'MaxAniso',
        'AntiAliasing',
        'MotionBlur',
        'ShadowDetail',
      ].includes(field.name),
  ),
)
const draftChanged = computed(() =>
  editableSettings.value.some((field) => draft.value[field.name] !== baseline.value[field.name]),
)
const changes = computed(() =>
  editableSettings.value
    .filter(
      (setting) =>
        typeof draft.value[setting.name] === 'number' &&
        draft.value[setting.name] !== baseline.value[setting.name],
    )
    .map((setting) => ({ name: setting.name, value: draft.value[setting.name]! })),
)
const dirty = computed(
  () => draftChanged.value || selectedProfile.value !== null || preview.value !== null,
)
const changeDetails = computed(() => [
  ...(selectedProfile.value
    ? [t('graphicsProfile') + ' · ' + t('profile' + selectedProfile.value)]
    : []),
  ...editableSettings.value
    .filter((field) => draft.value[field.name] !== baseline.value[field.name])
    .map((field) => t('setting' + field.name)),
])
const draftProfile = computed(() =>
  draftChanged.value
    ? 'custom'
    : (selectedProfile.value ?? report.value?.currentProfile ?? 'custom'),
)
const { leaving, finish } = useDraftNavigation(dirty, busy)

function discard() {
  selectedProfile.value = null
  baseline.value = Object.fromEntries(
    editableSettings.value.map((field) => [field.name, field.value]),
  )
  draft.value = { ...baseline.value }
  done.value = false
}
function chooseProfile(profile: GraphicsProfile) {
  if (busy.value) return
  const recipe = report.value?.profiles.find((item) => item.name === profile && item.available)
  if (!recipe) return
  selectedProfile.value = profile === report.value?.currentProfile ? null : profile
  const values = Object.fromEntries(
    editableSettings.value.map((field) => [field.name, field.value]),
  )
  for (const setting of recipe.settings) {
    if (setting.name in values) values[setting.name] = setting.value
  }
  baseline.value = values
  const next = { ...draft.value }
  for (const setting of recipe.settings) {
    if (setting.name in next) next[setting.name] = setting.value
  }
  draft.value = next
  done.value = false
}

function dword(image: { bytes: number[] } | null) {
  return image?.bytes.length === 4
    ? new DataView(Uint8Array.from(image.bytes).buffer).getUint32(0, true)
    : t('settingAbsent')
}

function displayedValue(name: string, image: { bytes: number[] } | null) {
  const value = dword(image)
  return name === 'Language' && typeof value === 'number'
    ? `${report.value?.gameLanguages.find((language) => language.value === value)?.nativeName ?? value} (${value})`
    : value
}

async function load() {
  report.value = await desktop.nativeSettings()
  discard()
  operation.value = await desktop.settingsOperation()
  preview.value = operation.value?.phase === 'prepared' ? operation.value : null
}
async function verify() {
  phase.value = 'verifying'
  const root = store.report?.root
  verificationFailed.value =
    !root ||
    !(await store.scanWhenIdle(root)) ||
    !!store.report?.checks.some((check) => check.status === 'error')
  return !verificationFailed.value
}
async function apply() {
  if (busy.value || store.busy) return
  const form = document.getElementById('graphics-form') as HTMLFormElement | null
  if (!preview.value && form && !form.checkValidity()) {
    if (leaving.value) {
      finish(false)
      await nextTick()
    }
    form.reportValidity()
    return
  }
  busy.value = true
  phase.value = 'working'
  store.error = null
  done.value = false
  try {
    if (!dirty.value && verificationFailed.value) {
      done.value = await verify()
      if (done.value && leaving.value) finish(true)
      return
    }
    const plan =
      preview.value ??
      (await desktop.prepareSettings(changes.value, selectedProfile.value ?? undefined))
    await desktop.applySettings(plan.digest)
    preview.value = null
    await load()
    const verified = await verify()
    done.value = verified
    if (verified && leaving.value) finish(true)
  } catch (failure) {
    store.error = errorCode(failure)
  } finally {
    busy.value = false
  }
}
async function cancelChanges() {
  if (busy.value) return
  if (preview.value) {
    busy.value = true
    try {
      await desktop.restoreSettings(preview.value.digest)
      preview.value = null
    } catch (failure) {
      store.error = errorCode(failure)
      return
    } finally {
      busy.value = false
    }
  }
  discard()
  if (leaving.value) finish(true)
}
async function restore(digest: string) {
  if (busy.value) return
  busy.value = true
  store.error = null
  try {
    await desktop.restoreSettings(digest)
    await load()
    await verify()
  } catch (failure) {
    store.error = errorCode(failure)
  } finally {
    busy.value = false
  }
}
async function initialize() {
  if (busy.value) return
  busy.value = true
  phase.value = 'loading'
  store.error = null
  try {
    await load()
  } catch (failure) {
    store.error = errorCode(failure)
  } finally {
    busy.value = false
  }
}
onMounted(initialize)
</script>

<template>
  <div class="page">
    <header class="page-heading">
      <p class="eyebrow">{{ t('graphics') }}</p>
      <h1 tabindex="-1">{{ t('graphicsTitle') }}</h1>
      <p class="intro">{{ t('graphicsDescription') }}</p>
    </header>
    <DraftBar
      :dirty="dirty"
      :details="changeDetails"
      :busy="busy && phase !== 'loading'"
      @apply="apply"
      @discard="cancelChanges"
    />
    <BusyState v-if="busy" :label="ui(phase)" />
    <AppButton v-if="!report && !busy" @click="initialize">{{ t('retry') }}</AppButton>
    <p v-if="verificationFailed" role="alert" class="notice notice--error">
      {{ ui('verifyFailed') }} <RouterLink to="/repair">{{ t('viewDiagnostics') }}</RouterLink>
    </p>

    <p v-if="done" role="status">{{ t('settingsApplied') }}</p>
    <section v-if="report?.profiles?.length" class="section profile-panel">
      <h2>{{ t('graphicsProfile') }} · {{ t('profile' + draftProfile) }}</h2>
      <p>
        {{
          report.profiles.some((profile) => profile.available)
            ? ui('profileHint')
            : t(report.buildSupported ? 'profilesUnavailable' : 'channelUnqualified')
        }}
      </p>
      <div class="plan-actions">
        <AppButton
          v-for="profile in report.profiles"
          :key="profile.name"
          :disabled="!profile.available || busy || !!preview"
          :aria-pressed="draftProfile === profile.name"
          @click="chooseProfile(profile.name)"
          >{{ t('profile' + profile.name) }}</AppButton
        >
      </div>
    </section>
    <form v-if="report" id="graphics-form" @input="done = false" @submit.prevent="apply">
      <template v-for="group in settingGroups" :key="group.key">
        <fieldset v-if="group.fields.length" class="settings-group">
          <legend>{{ ui(group.key) }}</legend>
          <GraphicsFields v-model="draft" :fields="group.fields" :disabled="busy || !!preview" />
        </fieldset>
      </template>
      <details v-if="advancedFields.length" class="settings-group advanced-graphics">
        <summary>{{ ui('advancedGraphics') }}</summary>
        <p class="field-hint">{{ ui('advancedHint') }}</p>
        <GraphicsFields v-model="draft" :fields="advancedFields" :disabled="busy || !!preview" />
      </details>
    </form>
    <details v-if="inspectedSettings.length" class="technical-details">
      <summary>{{ t('inspectedSettings') }}</summary>
      <dl class="installation-details">
        <div v-for="setting in inspectedSettings" :key="setting.name">
          <dt>{{ t('setting' + setting.name) }}</dt>
          <dd :id="'setting-' + setting.name">
            {{
              setting.value ??
              (setting.valueType === null ? t('settingAbsent') : t('settingOpaque'))
            }}
          </dd>
        </div>
      </dl>
    </details>
    <section v-if="preview" class="section review-panel" aria-labelledby="plan-title" role="region">
      <h2 id="plan-title">{{ t('settingsPlan') }}</h2>
      <p>{{ t('settingsPlanHint') }}</p>
      <ul>
        <li v-for="action in preview.plan.actions" :key="action.target.name">
          {{ t('setting' + action.target.name) }} :
          {{ displayedValue(action.target.name, action.before) }} →
          {{ displayedValue(action.target.name, action.after) }}
        </li>
      </ul>
      <div class="plan-actions">
        <AppButton variant="primary" :loading="busy" @click="apply">{{
          t('approveSettings')
        }}</AppButton>
        <AppButton :disabled="busy" @click="cancelChanges">{{ t('cancel') }}</AppButton>
      </div>
    </section>
    <details
      v-if="operation && !preview && !['rolledBack', 'cancelled'].includes(operation.phase)"
      class="technical-details"
    >
      <summary>{{ t('restoreSettings') }}</summary>
      <p>{{ t('restoreSettingsHint') }}</p>
      <ul>
        <li v-for="action in operation.plan.actions" :key="action.target.name">
          {{ t('setting' + action.target.name) }} :
          {{ displayedValue(action.target.name, action.after) }} →
          {{ displayedValue(action.target.name, action.before) }}
        </li>
      </ul>
      <AppButton :loading="busy" @click="restore(operation.digest)">{{
        t('restoreSettings')
      }}</AppButton>
    </details>
    <details class="technical-details">
      <summary>{{ t('profilesAndLanguages') }}</summary>
      <p>{{ t('profilesPending') }}</p>
      <ul v-if="report?.gameLanguages?.length">
        <li v-for="language in report.gameLanguages" :key="language.code">
          {{ language.nativeName }} ·
          {{ language.available ? t('languageResourcePresent') : t('languageResourcesMissing') }}
        </li>
      </ul>
    </details>
    <details v-if="report?.capabilities?.length" class="technical-details">
      <summary>{{ t('capabilityMatrix') }}</summary>
      <p>{{ t('settingsExperimental') }}</p>
      <p>{{ t('profileNotice') }}</p>
      <dl class="installation-details">
        <div v-for="capability in report.capabilities" :key="capability.name">
          <dt>{{ t('setting' + capability.name) }}</dt>
          <dd>
            {{ t('capabilityKind' + capability.kind) }} ·
            {{ t('capabilityState' + capability.state) }}<br />
            {{ t(capability.reason) }}
          </dd>
        </div>
      </dl>
    </details>
    <DraftLeaveDialog
      :open="leaving"
      :details="changeDetails"
      :busy="busy"
      :error="store.error ? t(store.error) : verificationFailed ? ui('verifyFailed') : ''"
      @apply="apply"
      @discard="cancelChanges"
      @stay="finish(false)"
    />
  </div>
</template>
