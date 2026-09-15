import { createApp } from "vue";
import { createPinia } from "pinia";
import { bildir } from "./lib/gunluk";
import App from "./App.vue";
import { router } from "./router";
import "./styles/base.css";

window.addEventListener("error", (e) => {
  bildir("hata", `${e.message} @ ${e.filename}:${e.lineno}:${e.colno}`);
});
window.addEventListener("unhandledrejection", (e) => {
  const neden = e.reason as { stack?: string } | undefined;
  bildir("hata", `yakalanmamış söz: ${neden?.stack ?? String(e.reason)}`);
});

const app = createApp(App);
app.config.errorHandler = (err, _ornek, bilgi) => {
  const hata = err as { stack?: string } | undefined;
  bildir("hata", `${bilgi}: ${hata?.stack ?? String(err)}`);
};

app.use(createPinia()).use(router).mount("#app");
