// vue
import { createApp } from 'vue';
import App from './App.vue';

// external js libraries
import axios from 'axios';
import semver from 'semver';
import { Tooltip, Popover } from 'bootstrap';

// css
import 'bootstrap/dist/css/bootstrap.min.css';

// ---

// create vue app
const app = createApp(App);

// create a v-tooltip="'some text'" directive
app.directive('tooltip', {
  mounted(el, binding) {
    new Tooltip(el, {
      placement: 'top',
      trigger: 'hover focus', // 'click', 
      title: binding.value,
      //      boundary: 'viewport',
      container: 'body',
    });
  }
})


// create a v-popover="'some text'" directive
app.directive('popover', {
  mounted(el, binding) {
    new Popover(el, {
      content: binding.value,
    });
  }
})

// load external libraries
app.provide('axios', axios);
app.provide('semver', semver);

// mount
app.mount('#app');

