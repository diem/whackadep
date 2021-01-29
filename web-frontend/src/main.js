import { createApp } from 'vue';
import App from './App.vue';

import axios from 'axios';

import 'bootstrap/dist/css/bootstrap.min.css';

// this works, but seems not recommended (https://vuejs.org/v2/cookbook/adding-instance-properties.html)
// global.bootstrap = bootstrap;

const app = createApp(App);

// https://stackoverflow.com/questions/65184107/how-to-use-vue-prototype-or-global-variable-in-vue-3
app.provide('axios', axios);

app.mount('#app');

