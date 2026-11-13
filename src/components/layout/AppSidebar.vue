<script setup lang="ts">
import { House, SlidersHorizontal, Layers, Archive, Wrench, Settings } from '@lucide/vue'
import logo from '../../assets/brand/arachnid-logo.png'
import { t } from '../../composables/i18n'
import { ui } from '../../composables/interfaceMessages'
import packageInfo from '../../../package.json'
const items = [
  { key: 'home', path: '/home', icon: House },
  { key: 'graphics', path: '/graphics', icon: SlidersHorizontal },
  { key: 'mods', path: '/mods', icon: Layers },
  { key: 'saves', path: '/saves', icon: Archive },
  { key: 'repair', path: '/repair', icon: Wrench },
  { key: 'settings', path: '/settings', icon: Settings },
]
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <img :src="logo" alt="" width="32" height="32" /><span
        >Arachnid<small>TASM · 2012</small></span
      >
    </div>
    <nav :aria-label="ui('navigation')">
      <template v-for="item in items" :key="item.key">
        <RouterLink
          v-if="item.path"
          :to="item.path"
          class="nav-item"
          active-class="nav-item--active"
        >
          <component :is="item.icon" :size="18" aria-hidden="true" />{{ t(item.key) }}
        </RouterLink>
        <span v-else class="nav-item nav-item--unavailable" :title="t('later')">
          <component :is="item.icon" :size="18" aria-hidden="true" /><span
            >{{ t(item.key) }}<small>{{ t('later') }}</small></span
          >
        </span>
      </template>
    </nav>
    <footer>
      <span class="version">v{{ packageInfo.version }}</span
      ><span>Windows</span>
    </footer>
  </aside>
</template>
