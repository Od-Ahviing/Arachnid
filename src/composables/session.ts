import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { desktop, errorCode } from '../services/desktop'
import { useInstallation } from '../stores/installation'
import type { LaunchStatus } from '../types/operations'
import { locale } from './i18n'

export function useSession() {
  const store = useInstallation()
  const status = ref<LaunchStatus | null>(null)
  const busy = ref(false)
  const canStart = computed(
    () =>
      store.report?.recognized &&
      !store.busy &&
      !store.languageBusy &&
      !store.graphicsModsBusy &&
      !status.value?.decision.reasons.includes('gameAlreadyRunning') &&
      !['running', 'unknown'].includes(status.value?.session.state ?? ''),
  )
  let timer: ReturnType<typeof setInterval> | undefined
  let refreshing = false
  let active = true
  let revision = 0

  async function refresh() {
    if (refreshing || busy.value || store.busy || store.languageBusy || !active) return
    refreshing = true
    const requestRevision = ++revision
    try {
      const value = await desktop.launchStatus()
      if (active && requestRevision === revision) status.value = value
    } catch (failure) {
      if (requestRevision !== revision) return
      const code = errorCode(failure)
      if (code !== 'scanBusy' && code !== 'installationBusy') {
        status.value = null
        store.error ??= code
      }
    } finally {
      refreshing = false
    }
  }

  async function start() {
    if (busy.value || !canStart.value) return
    busy.value = true
    revision++
    store.error = null
    try {
      status.value = await desktop.startGame(locale.value, (report) => {
        if (store.report?.root === report.root) store.report = report
      })
    } catch (failure) {
      store.error = errorCode(failure)
      status.value = null
    } finally {
      busy.value = false
    }
    if (!status.value) void refresh()
  }

  onMounted(() => {
    void refresh()
    timer = setInterval(() => void refresh(), 2000)
  })
  watch(
    () => store.report,
    () => void refresh(),
  )
  onUnmounted(() => {
    active = false
    clearInterval(timer)
  })
  return { status, busy, canStart, start }
}
