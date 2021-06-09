import Vue from 'vue'
import Router from 'vue-router'

import LandingPage from '@/components/LandingPage'
import Repo from '@/components/Repo'
import Dashboard from '@/components/Dashboard'
import Review from '@/components/Review'
import DependencyHealthDashboard from '@/components/DependencyHealthDashboard'

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
      component: Repo,
      props: route => ({ repo: route.params.repo }),
      children: [
        // UserHome will be rendered inside User's <router-view>
        // when /user/:id is matched
        {
          path: '',
          name: 'repo',
          component: Dashboard,
        },
        {
          path: 'review/:depkey',
          name: 'review',
          component: Review,
          props: route => ({ depkey: route.params.depkey }),
        },
        {
          path: 'dependency_health',
          name: 'dependency_health',
          component: DependencyHealthDashboard,
        }
      ],
    },

  ]
})
