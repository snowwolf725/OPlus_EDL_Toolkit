import { ref, createApp } from "vue";
import { createI18n } from 'vue-i18n';
// Vuetify
import 'vuetify/styles'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import * as labsComponents from 'vuetify/labs/components'
// Components
import App from './App.vue'
import en from "./locales/en.js";
import ru from "./locales/ru.js";
import zh_CN from "./locales/zh_CN";
import zh_TW from "./locales/zh_TW";

const i18n = createI18n({
    local: 'en',
    fallbackLocale: 'en',
    legacy: false,
    messages: {
        en: en,
        ru: ru,
        zh_CN: zh_CN,
        zh_TW: zh_TW,
    }
});

const vuetify = createVuetify({
    components: {
        ...components,
        ...labsComponents,
    },
    directives,
});

const app = createApp(App);
app.use(i18n).use(vuetify);
app.mount('#app');
//createApp(App).mount("#app");
