import { inject, onBeforeUnmount, onMounted, ref, type Ref } from 'vue'
import { matchedRouteKey, onBeforeRouteLeave } from 'vue-router'

export function useDraftNavigation(dirty: Readonly<Ref<boolean>>, busy: Readonly<Ref<boolean>>) {
  const leaving = ref(false)
  let resolveNavigation: ((proceed: boolean) => void) | undefined

  function finish(proceed: boolean) {
    leaving.value = false
    resolveNavigation?.(proceed)
    resolveNavigation = undefined
  }

  if (inject(matchedRouteKey, null)) {
    onBeforeRouteLeave(() => {
      if (busy.value) return false
      if (!dirty.value) return true
      finish(false)
      leaving.value = true
      return new Promise<boolean>((resolve) => {
        resolveNavigation = resolve
      })
    })
  }

  function beforeUnload(event: BeforeUnloadEvent) {
    if (!dirty.value && !busy.value) return
    event.preventDefault()
    event.returnValue = ''
  }
  onMounted(() => window.addEventListener('beforeunload', beforeUnload))
  onBeforeUnmount(() => {
    finish(false)
    window.removeEventListener('beforeunload', beforeUnload)
  })
  return { leaving, finish }
}
