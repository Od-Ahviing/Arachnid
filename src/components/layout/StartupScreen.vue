<script setup lang="ts">
import logo from '../../assets/brand/arachnid-logo.png'
import loadingLogo from '../../assets/brand/arachnid-loading.gif'
import { t } from '../../composables/i18n'
</script>

<template>
  <div class="startup-screen" role="status" aria-live="polite" aria-busy="true">
    <picture class="startup-mark">
      <source media="(prefers-reduced-motion: reduce)" :srcset="logo" />
      <img :src="loadingLogo" alt="" width="192" height="192" />
    </picture>
    <strong>Arachnid</strong>
    <p>{{ t('loading') }}</p>
  </div>
</template>

<style scoped>
.startup-screen {
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  text-align: center;
}
.startup-mark {
  display: grid;
  place-items: center;
  width: 192px;
  height: 192px;
  margin-bottom: 0.5rem;
}
.startup-mark img {
  object-fit: contain;
}
strong {
  font: 500 1.3rem var(--font-display);
  letter-spacing: 0.03em;
}
p {
  margin-top: 0.75rem;
  font-size: var(--text-caption);
}
@media (prefers-reduced-motion: reduce) {
  .startup-mark img {
    width: 150px;
    height: 150px;
  }
}
</style>
