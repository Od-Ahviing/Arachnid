import { createRouter, createWebHashHistory } from 'vue-router'
import { useInstallation } from '../stores/installation'

export const router = createRouter({
  history: createWebHashHistory(),
  scrollBehavior(_to, _from, savedPosition) {
    return savedPosition ?? { top: 0 }
  },
  routes: [
    { path: '/', redirect: '/setup' },
    { path: '/setup', component: () => import('../views/SetupView.vue') },
    { path: '/home', component: () => import('../views/HomeView.vue') },
    { path: '/graphics', component: () => import('../views/GraphicsView.vue') },
    { path: '/mods', component: () => import('../views/ModsView.vue') },
    { path: '/saves', component: () => import('../views/SavesView.vue') },
    { path: '/repair', component: () => import('../views/RepairView.vue') },
    { path: '/settings', component: () => import('../views/SettingsView.vue') },
    { path: '/:pathMatch(.*)*', redirect: '/setup' },
  ],
})

router.beforeEach((to) => {
  if (to.path !== '/setup' && !useInstallation().report?.recognized) return '/setup'
})
