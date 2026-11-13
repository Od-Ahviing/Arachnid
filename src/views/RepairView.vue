<script setup lang="ts">
import { computed, ref } from 'vue'
import { RefreshCw, Download } from '@lucide/vue'
import { useInstallation } from '../stores/installation'
import { desktop, errorCode } from '../services/desktop'
import { t } from '../composables/i18n'
import AppButton from '../components/ui/AppButton.vue'
import CheckList from '../components/diagnostics/CheckList.vue'
import { ui } from '../composables/interfaceMessages'
const filter = ref<'all' | 'attention' | 'unverified'>('attention')
const store = useInstallation()
const launchChecks = ['direct3d', 'legacyDirectx', 'visualCpp', 'gameBuild']
const issues = computed(
  () =>
    store.report?.checks.filter(
      (check) =>
        ['error', 'warning'].includes(check.status) ||
        (check.status === 'unverified' && launchChecks.includes(check.code)),
    ) ?? [],
)
const visibleChecks = computed(() =>
  filter.value === 'all'
    ? (store.report?.checks ?? [])
    : filter.value === 'unverified'
      ? unverified.value
      : issues.value,
)
const unverified = computed(
  () =>
    store.report?.checks.filter((check) => ['unverified', 'skipped'].includes(check.status)) ?? [],
)
const exported = ref(false)
const exportBusy = ref(false)
async function saveReport() {
  exportBusy.value = true
  try {
    exported.value = await desktop.exportReport()
  } catch (failure) {
    store.error = errorCode(failure)
  } finally {
    exportBusy.value = false
  }
}
</script>

<template>
  <div v-if="store.report" class="page">
    <header class="page-heading">
      <p class="eyebrow">{{ t('repair') }}</p>
      <h1 tabindex="-1">{{ t('checksTitle') }}</h1>
      <p class="intro">{{ t('checksDescription') }}</p>
      <AppButton :loading="store.busy" @click="store.scanWhenIdle(store.report.root)"
        ><RefreshCw :size="16" aria-hidden="true" />{{ t('scanAgain') }}</AppButton
      >
    </header>
    <div class="section-switcher" :aria-label="t('checksTitle')">
      <button type="button" :aria-pressed="filter === 'attention'" @click="filter = 'attention'">
        {{ ui('attentionChecks') }} <span class="count-badge">{{ issues.length }}</span>
      </button>
      <button type="button" :aria-pressed="filter === 'all'" @click="filter = 'all'">
        {{ ui('allChecks') }} <span class="count-badge">{{ store.report.checks.length }}</span>
      </button>
      <button type="button" :aria-pressed="filter === 'unverified'" @click="filter = 'unverified'">
        {{ ui('notVerified') }} <span class="count-badge">{{ unverified.length }}</span>
      </button>
    </div>
    <CheckList v-if="visibleChecks.length" :checks="visibleChecks" />
    <p v-else class="empty-state" role="status">{{ ui('noIssues') }}</p>
    <details v-if="store.report.binaries.length" class="technical-details">
      <summary>{{ t('technical') }}</summary>
      <div v-for="binary in store.report.binaries" :key="binary.name" class="binary-row">
        <h3>{{ binary.name }}</h3>
        <p>{{ t('architecture') }} : {{ binary.architecture }}</p>
        <p>{{ t('hash') }}</p>
        <code>{{ binary.sha256 }}</code>
        <p>{{ t('imports') }}</p>
        <code v-if="binary.importsInspected">{{ binary.imports.join(', ') }}</code>
        <p v-else>{{ t('importsOmitted') }}</p>
      </div>
    </details>
    <section class="section export-section">
      <h2>{{ t('exportTitle') }}</h2>
      <p>{{ t('exportHint') }}</p>
      <AppButton :disabled="store.busy" :loading="exportBusy" @click="saveReport"
        ><Download :size="16" aria-hidden="true" />{{ t('export') }}</AppButton
      >
      <p v-if="exported" role="status">{{ t('exportDone') }}</p>
    </section>
  </div>
</template>
