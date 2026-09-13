//! Arayüze olay yollamak için genel bir kanal.
//!
//! `engines` gibi derin modüllerin `AppHandle` taşımasına gerek kalmasın diye
//! uygulama başlarken bir kez saklanıyor.

use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Emitter};

fn handle() -> &'static Mutex<Option<AppHandle>> {
    static HANDLE: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();
    HANDLE.get_or_init(|| Mutex::new(None))
}

pub fn set_handle(app: AppHandle) {
    if let Ok(mut guard) = handle().lock() {
        *guard = Some(app);
    }
}

/// Arayüze olay yollar. Uygulama henüz hazır değilse sessizce geçer —
/// bildirimin kaybolması, işin durmasından iyidir.
pub fn emit(event: &str, payload: impl serde::Serialize + Clone) {
    if let Ok(guard) = handle().lock() {
        if let Some(app) = guard.as_ref() {
            let _ = app.emit(event, payload);
        }
    }
}
