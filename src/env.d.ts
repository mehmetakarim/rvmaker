/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

/** Vite tarafından derleme anında gömülür. */
declare const __BUILD_TIME__: string;
