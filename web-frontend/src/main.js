// vue
import { createApp } from 'vue';
import App from './App.vue';

// external js libraries
import axios from 'axios';
import semver from 'semver';
import { Tooltip } from 'bootstrap';

// css
import 'bootstrap/dist/css/bootstrap.min.css';

const app = createApp(App);
app.directive('tooltip', {
  mounted(el) {
    new Tooltip(el, {
      boundary: 'window',
    });
  }
})
app.provide('axios', axios);
app.provide('semver', semver);
app.mount('#app');

