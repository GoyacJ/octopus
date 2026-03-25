import { createRouter, createWebHistory } from 'vue-router'

import ShellHomeView from '@/views/ShellHomeView.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: ShellHomeView,
    },
  ],
})

