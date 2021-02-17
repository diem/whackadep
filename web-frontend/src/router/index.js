import Vue from 'vue'
import Router from 'vue-router'

import LandingPage from '@/components/LandingPage'
import DashBoard from '@/components/Dashboard'

Vue.use(Router)

export default new Router({
  routes: [
    {
      path: '/',
      name: 'landing',
      component: LandingPage
    },
    {
      path: '/repo/:repo',
      name: 'repo',
      component: DashBoard,
      props: route => ({ repo: route.params.repo }),
    }
  ]
})
