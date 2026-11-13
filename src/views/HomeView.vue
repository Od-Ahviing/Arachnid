<script lang="ts">
import { ref } from 'vue'
const dismissedCrashPid = ref<number | null>(null)
</script>

<script setup lang="ts">
import { computed, watch } from 'vue'
import {
  ArrowUpRight,
  Play,
  SlidersHorizontal,
  Layers,
  Archive,
  CircleCheck,
  CircleAlert,
} from '@lucide/vue'
import { useInstallation } from '../stores/installation'
import { locale, t } from '../composables/i18n'
import AppButton from '../components/ui/AppButton.vue'
import ActionDialog from '../components/ui/ActionDialog.vue'
import { useSession } from '../composables/session'
import { ui } from '../composables/interfaceMessages'
import GameLanguageControl from '../components/game/GameLanguageControl.vue'
const shortcuts = [
  { path: '/graphics', key: 'graphics', hint: 'graphicsHint', icon: SlidersHorizontal },
  { path: '/mods', key: 'mods', hint: 'modsHint', icon: Layers },
  { path: '/saves', key: 'saves', hint: 'savesHint', icon: Archive },
] as const
const store = useInstallation()
const { status, busy, start } = useSession()
const crashPid = computed(() => status.value?.session.pid ?? null)
const showCrashAdvice = computed(
  () =>
    !!status.value?.graphicsCrashAdvice &&
    crashPid.value !== null &&
    crashPid.value !== dismissedCrashPid.value,
)
watch(
  () => status.value?.session.state,
  (state) => {
    if (state === 'running') dismissedCrashPid.value = null
  },
)
function dismissCrashAdvice() {
  dismissedCrashPid.value = crashPid.value
}
</script>

<template>
  <div v-if="store.report" class="page home-page">
    <ActionDialog
      v-if="showCrashAdvice"
      :open="true"
      :title="t('graphicsCrashTitle')"
      :busy="false"
      @close="dismissCrashAdvice"
    >
      <p>
        {{
          t(
            status?.graphicsCrashAdvice === 'reshade'
              ? 'graphicsCrashReshade'
              : 'graphicsCrashMods',
          )
        }}
      </p>
      <div class="dialog-actions">
        <AppButton variant="quiet" @click="dismissCrashAdvice">{{
          t('graphicsCrashClose')
        }}</AppButton>
        <RouterLink v-slot="{ navigate }" to="/mods" custom>
          <AppButton
            variant="primary"
            @click="
              () => {
                dismissCrashAdvice()
                navigate()
              }
            "
          >
            {{ t('graphicsCrashOpenMods') }}
          </AppButton>
        </RouterLink>
      </div>
    </ActionDialog>
    <section class="home-hero">
      <header class="home-heading">
        <p class="eyebrow">{{ t('gameEdition') }}</p>
        <h1 tabindex="-1">
          <span class="game-title-line">The Amazing</span
          ><span class="game-title-line">Spider-Man</span>
        </h1>
        <p class="intro">{{ ui('homeHint') }}</p>
        <span
          class="readiness"
          :class="{ 'readiness--ready': status?.decision.ready }"
          role="status"
        >
          <component
            :is="status?.decision.ready ? CircleCheck : CircleAlert"
            :size="16"
            aria-hidden="true"
          />
          {{ ui(!status ? 'checking' : status.decision.ready ? 'ready' : 'attention') }}
        </span>
      </header>
      <div class="play-section">
        <AppButton
          variant="primary"
          :disabled="!status?.decision.ready || store.busy || store.languageBusy"
          :loading="busy"
          @click="start"
          ><Play :size="17" aria-hidden="true" />{{ t('play') }}</AppButton
        >
        <p v-if="busy" role="status">{{ t('launching') }}</p>
        <p v-else-if="status?.session.state === 'running'" role="status">{{ t('gameRunning') }}</p>
        <p v-else-if="status?.session.state === 'exited'" role="status">
          {{ t('gameExited') }} {{ status.session.exitCode ?? t('exitCodeUnknown') }}
        </p>
        <p v-else>{{ ui('lifecycle') }}</p>
      </div>
      <ul v-if="status && !status.decision.ready" class="notice">
        <li v-for="reason in status.decision.reasons" :key="reason">{{ t(reason) }}</li>
      </ul>
    </section>
    <GameLanguageControl />
    <div class="home-shortcuts">
      <RouterLink
        v-for="shortcut in shortcuts"
        :key="shortcut.key"
        :to="shortcut.path"
        class="shortcut-card"
      >
        <component :is="shortcut.icon" :size="22" aria-hidden="true" />
        <h2>{{ t(shortcut.key) }}</h2>
        <p>{{ ui(shortcut.hint) }}</p>
        <ArrowUpRight :size="17" class="shortcut-arrow" aria-hidden="true" />
      </RouterLink>
    </div>
    <details class="section technical-details home-installation">
      <summary>{{ t('overview') }}</summary>
      <div class="section-heading">
        <h2>{{ t('overview') }}</h2>
        <RouterLink to="/repair" class="text-link"
          >{{ t('viewDiagnostics') }}<ArrowUpRight :size="16" aria-hidden="true"
        /></RouterLink>
      </div>
      <dl class="installation-details">
        <div>
          <dt>{{ t('folder') }}</dt>
          <dd class="path-value">{{ store.report.root }}</dd>
        </div>
        <div>
          <dt>{{ t('files') }}</dt>
          <dd>{{ new Intl.NumberFormat(locale).format(store.report.fileCount) }}</dd>
        </div>
        <div>
          <dt>{{ t('archives') }}</dt>
          <dd>{{ store.report.pkzCount }}</dd>
        </div>
        <div>
          <dt>{{ t('size') }}</dt>
          <dd>
            {{
              new Intl.NumberFormat(locale, {
                style: 'unit',
                unit: 'gigabyte',
                maximumFractionDigits: 2,
              }).format(store.report.totalBytes / 1_000_000_000)
            }}
          </dd>
        </div>
        <div>
          <dt>{{ t('lastScan') }}</dt>
          <dd>
            {{
              new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(
                store.report.scannedAt * 1000,
              )
            }}
          </dd>
        </div>
      </dl>
    </details>
  </div>
</template>
