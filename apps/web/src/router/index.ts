import { createRouter, createWebHistory } from 'vue-router'
import AgentsPage from '@/pages/AgentsPage.vue'
import AuditPage from '@/pages/AuditPage.vue'
import InboxPage from '@/pages/InboxPage.vue'
import OverviewPage from '@/pages/OverviewPage.vue'
import RunsPage from '@/pages/RunsPage.vue'
import WorkspacesPage from '@/pages/WorkspacesPage.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: OverviewPage },
    { path: '/workspaces', component: WorkspacesPage },
    { path: '/agents', component: AgentsPage },
    { path: '/runs', component: RunsPage },
    { path: '/inbox', component: InboxPage },
    { path: '/audit', component: AuditPage },
  ],
})
