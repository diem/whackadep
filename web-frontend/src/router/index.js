import Vue from 'vue'
import Router from 'vue-router'

import LandingPage from '@/components/LandingPage'
import DashBoard from '@/components/Dashboard'
import Review from '@/components/Review'

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
    },
    {
      path: '/repo/:repo/review/:depkey',
      name: 'review',
      component: Review,
      props: route => ({ repo: route.params.repo, depkey: route.params.depkey }),
    }
  ]
})
