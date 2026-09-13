import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";

/** Store henüz kurulmadan okunduğu için doğrudan depolamadan bakıyoruz. */
function localStorageStartRoute(): string {
  try {
    const value = localStorage.getItem("rv-start-on-launch");
    if (value === "library") return "/kitaplik";
    if (value === "setup") return "/kurulum";
  } catch {
    /* depolama kapalı olabilir */
  }
  return "/yeni/kaynak";
}

const routes: RouteRecordRaw[] = [
  // Kök rota, Ayarlar → Genel → "Açılışta göster" tercihine göre yönlenir.
  { path: "/", redirect: () => localStorageStartRoute() },
  {
    path: "/kurulum",
    name: "setup",
    component: () => import("@/views/SetupView.vue"),
    meta: { chrome: "bare", title: "Kurulum" },
  },
  {
    path: "/yeni",
    component: () => import("@/views/new/NewVideoLayout.vue"),
    // Kenar çubuğu "/yeni" adresine gidiyor; alt rota olmadan düzen boş kalırdı.
    redirect: { name: "new-source" },
    children: [
      {
        path: "kaynak",
        name: "new-source",
        component: () => import("@/views/new/SourceStep.vue"),
        meta: { step: 0, title: "Yeni Video" },
      },
      {
        path: "subreddit",
        name: "new-browse",
        component: () => import("@/views/new/SubredditStep.vue"),
        meta: { step: 0, title: "Yeni Video · Subreddit" },
      },
      {
        path: "viral",
        name: "new-viral",
        component: () => import("@/views/new/ViralStep.vue"),
        meta: { step: 0, title: "Yeni Video · X'te viral" },
      },
      {
        path: "icerik",
        name: "new-content",
        component: () => import("@/views/new/ContentStep.vue"),
        meta: { step: 1, title: "Yeni Video · İçerik" },
      },
      {
        path: "ses",
        name: "new-voice",
        component: () => import("@/views/new/VoiceStep.vue"),
        meta: { step: 2, title: "Yeni Video · Ses" },
      },
      {
        path: "gorunum",
        name: "new-look",
        component: () => import("@/views/new/LookStep.vue"),
        meta: { step: 3, title: "Yeni Video · Görünüm" },
      },
    ],
  },
  {
    path: "/render/:id?",
    name: "render",
    component: () => import("@/views/RenderView.vue"),
    meta: { title: "Üretim" },
  },
  {
    path: "/kuyruk",
    name: "queue",
    component: () => import("@/views/QueueView.vue"),
    meta: { title: "Kuyruk" },
  },
  {
    path: "/kitaplik",
    name: "library",
    component: () => import("@/views/LibraryView.vue"),
    meta: { title: "Kitaplık" },
  },
  {
    path: "/ayarlar/:tab?",
    name: "settings",
    component: () => import("@/views/SettingsView.vue"),
    meta: { title: "Ayarlar" },
  },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});
