import '@babel/polyfill'
import 'mutationobserver-shim'

import Vue from 'vue'
import App from './App.vue'

// disable warnings (https://stackoverflow.com/questions/41743926/disable-development-mode-warning-in-vuejs)
Vue.config.productionTip = false

// set up icons with bootstrap
import './plugins/bootstrap-vue'
import { BootstrapVue, BootstrapVueIcons } from 'bootstrap-vue'
Vue.use(BootstrapVue)
Vue.use(BootstrapVueIcons)

// set up router with vue-router
import router from './router'

// set up a global storage with vuex
import store from "./store";

// finally set up Vue
new Vue({
  render: h => h(App),
  router,
  store
}).$mount('#app')
