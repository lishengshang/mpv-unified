import { createRouter, createWebHashHistory } from "vue-router";
import ProfilesView from "../views/ProfilesView.vue";
import ConfigView from "../views/ConfigView.vue";
import StoreView from "../views/StoreView.vue";
import HelpView from "../views/HelpView.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/profiles" },
    { path: "/profiles", name: "profiles", component: ProfilesView },
    { path: "/config", name: "config", component: ConfigView },
    { path: "/store", name: "store", component: StoreView },
    { path: "/help", name: "help", component: HelpView },
  ],
});
