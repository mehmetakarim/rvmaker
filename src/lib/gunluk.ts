import { invoke } from "@tauri-apps/api/core";

/** Ön yüzden Rust'ın stderr'ine ileti yazar. Paketli uygulamada webview
 *  konsolu görünmediği için açılış sorunlarını izlemenin tek yolu bu. */
export function bildir(seviye: "hata" | "bilgi", mesaj: string) {
  if (seviye === "hata") console.error(mesaj);
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    invoke("frontend_log", { level: seviye, message: mesaj }).catch(() => {
      /* bildirimin kendisi düşerse yapacak bir şey yok */
    });
  }
}
