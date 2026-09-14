<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  SlidersHorizontal,
  Mic,
  Languages,
  Video,
  Terminal,
  Check,
  CircleAlert,
  Lock,
  Folder,
  ExternalLink,
  Cookie,
  KeyRound,
  TriangleAlert,
  Info,
} from "@lucide/vue";
import { useSettingsStore } from "@/stores/settings";
import { isTauri, pickDirectory, revealInFinder, shortenHome } from "@/lib/api";
import { useDraftStore } from "@/stores/draft";
import RvButton from "@/components/ui/RvButton.vue";
import RvSelect from "@/components/ui/RvSelect.vue";
import RvSegmented from "@/components/ui/RvSegmented.vue";
import RvSlider from "@/components/ui/RvSlider.vue";
import RvSwitch from "@/components/ui/RvSwitch.vue";
import RvTextField from "@/components/ui/RvTextField.vue";

const settings = useSettingsStore();
const draft = useDraftStore();
const route = useRoute();
const router = useRouter();

const tabs = [
  { id: "genel", label: "Genel", icon: SlidersHorizontal },
  { id: "tts", label: "Seslendirme", icon: Mic },
  { id: "ceviri", label: "Çeviri", icon: Languages },
  { id: "video", label: "Video", icon: Video },
  { id: "gelismis", label: "Gelişmiş", icon: Terminal },
];

const active = computed(() => (route.params.tab as string) || "genel");

onMounted(() => {
  settings.refreshCookieStatus();
  settings.refreshEngineKeys();
  settings.refreshMaintenance();
  settings.refreshXStatus();
});

function go(tab: string) {
  router.push(`/ayarlar/${tab}`);
}

/** Hangi Reddit erişim yolunun devrede olduğu. */
const redditAccess = computed(() => {
  if (settings.reddit.clientId.trim()) return "clientId";
  if (settings.redditCookieSaved) return "cookie";
  return "none";
});

/** Klasör seçtirir; seçilen yolu `~` ile kısaltıp ayara yazar. */
async function pickFolder(target: "output" | "backgrounds" | "music") {
  if (!isTauri()) return;
  const picked = await pickDirectory();
  if (!picked) return;

  const short = await shortenHome(picked).catch(() => picked);
  if (target === "output") settings.video.outputDir = short;
  else if (target === "backgrounds") settings.backgroundsDir = short;
  else settings.musicDir = short;
}

async function openFolder(path: string) {
  if (!isTauri()) return;
  try {
    await revealInFinder(path);
  } catch {
    /* klasör henüz oluşmamış olabilir */
  }
}

const engineOptions = computed(() =>
  settings.engines.map((e) => ({ value: e.id, label: e.label })),
);

const voiceOptions = computed(() =>
  draft.voices.length > 0
    ? draft.voices.map((v) => ({ value: v.id, label: `${v.name} · ${v.detail}` }))
    : [{ value: "", label: "Ses adımında yüklenir" }],
);

const fileNameSample = computed(() => {
  const now = new Date();
  const date = now.toISOString().slice(0, 10);
  return (
    settings.video.nameTemplate
      .replace("{tarih}", date)
      .replace("{subreddit}", "askreddit")
      .replace("{baslik-kisa}", "en-cok-yanilinan")
      .replace("{baslik}", "en-cok-yanilinan-sey-nedir") + ".mp4"
  );
});
</script>

<template>
  <div class="wrap">
    <nav class="rail" data-component="SettingsTabs">
      <div class="rail-label">Ayarlar</div>
      <button
        v-for="tab in tabs"
        :key="tab.id"
        type="button"
        class="tab"
        :class="{ on: active === tab.id }"
        @click="go(tab.id)"
      >
        <component :is="tab.icon" :size="15" class="tab-icon" />
        {{ tab.label }}
      </button>
    </nav>

    <main class="panel rv-scroll">
      <!-- GENEL -->
      <template v-if="active === 'genel'">
        <header>
          <h1>Genel</h1>
          <p>Reddit erişimi ve uygulamanın genel davranışı.</p>
        </header>

        <section>
          <span class="section-label">Reddit erişimi</span>

          <div class="access-state" :class="redditAccess">
            <KeyRound v-if="redditAccess === 'clientId'" :size="14" class="access-icon" />
            <Cookie v-else-if="redditAccess === 'cookie'" :size="14" class="access-icon" />
            <TriangleAlert v-else :size="14" class="access-icon" />
            <span v-if="redditAccess === 'clientId'">
              İstemci kimliği kullanılıyor — gönderi çekme etkin.
            </span>
            <span v-else-if="redditAccess === 'cookie'">
              Oturum çerezi kullanılıyor — gönderi çekme etkin. Çerez süresi dolarsa yenilemen
              gerekir.
            </span>
            <span v-else>
              Reddit kimliksiz erişimi kapattı. Gönderi çekebilmek için aşağıdaki iki yoldan
              birini kur.
            </span>
          </div>

          <div class="row top">
            <span class="row-label">İstemci kimliği</span>
            <div class="stack">
              <RvTextField
                v-model="settings.reddit.clientId"
                mono
                placeholder="örn. kd9Xa2_bQ1..."
              />
              <span class="note">
                Önerilen yol: kalıcıdır, süresi dolmaz.
                <strong>installed app</strong> türünde bir uygulama oluştur; gizli anahtar
                gerekmez.
              </span>
              <a
                class="ext-link"
                href="https://www.reddit.com/prefs/apps"
                target="_blank"
                rel="noreferrer"
              >
                Reddit uygulama ayarlarını aç
                <ExternalLink :size="12" />
              </a>
            </div>
          </div>

          <div class="row top">
            <span class="row-label">Oturum çerezi</span>
            <div class="stack">
              <div class="key-row">
                <RvTextField
                  v-model="settings.redditCookieDraft"
                  type="password"
                  mono
                  class="grow"
                  :placeholder="
                    settings.redditCookieSaved
                      ? 'Kayıtlı — değiştirmek için yeni çerezi yapıştır'
                      : 'reddit_session=...; token_v2=...'
                  "
                />
                <RvButton
                  variant="secondary"
                  size="sm"
                  :loading="settings.redditCookieBusy"
                  :disabled="!settings.redditCookieDraft.trim()"
                  @click="settings.saveRedditCookie()"
                >
                  Kaydet
                </RvButton>
                <RvButton
                  v-if="settings.redditCookieSaved"
                  variant="ghost"
                  size="sm"
                  @click="settings.removeRedditCookie()"
                >
                  Sil
                </RvButton>
              </div>

              <span v-if="settings.redditCookieError" class="key-status error">
                <CircleAlert :size="12" />
                {{ settings.redditCookieError }}
              </span>
              <span v-else-if="settings.redditCookieSaved" class="key-status ok">
                <Check :size="12" />
                Çerez anahtar zincirinde kayıtlı.
              </span>

              <span class="note">
                Uygulama oluşturamıyorsan bu yolu kullan. Tarayıcında reddit.com'da oturum
                açıkken geliştirici araçlarını aç → Network sekmesinde bir isteğe tıkla →
                istek başlıklarındaki <span class="rv-mono">Cookie</span> satırının tamamını
                kopyala. İstemci kimliği doluysa o öncelikli olur.
              </span>

              <div class="privacy">
                <Lock :size="14" class="privacy-icon" />
                Çerez sistem anahtar zincirine yazılır, arayüze bir daha okunmaz ve yalnızca
                Reddit'e giden isteklerde kullanılır. Hesabına tam erişim verdiği için
                kullanmadığında silmen iyi olur.
              </div>
            </div>
          </div>
        </section>

        <section class="divided">
          <span class="section-label">X (Twitter) erişimi</span>

          <div class="access-state" :class="settings.xCookieSaved ? 'cookie' : 'none'">
            <Cookie v-if="settings.xCookieSaved" :size="14" class="access-icon" />
            <Info v-else :size="14" class="access-icon" />
            <span v-if="settings.xCookieSaved">
              Oturum çerezi kayıtlı — tweet yanıtları çekilebilir.
            </span>
            <span v-else>
              Tweet <strong>içeriği</strong> kimliksiz okunabiliyor; ama
              <strong>yanıt zincirini</strong> çekmek için oturum çerezin gerekiyor.
              Yanıtlar olmadan videoda tek bir kart olur.
            </span>
          </div>

          <div class="row top">
            <span class="row-label">Oturum çerezi</span>
            <div class="stack">
              <div class="key-row">
                <RvTextField
                  v-model="settings.xCookieDraft"
                  type="password"
                  mono
                  class="grow"
                  :placeholder="
                    settings.xCookieSaved
                      ? 'Kayıtlı — değiştirmek için yeni çerezi yapıştır'
                      : 'auth_token=...; ct0=...'
                  "
                />
                <RvButton
                  variant="secondary"
                  size="sm"
                  :loading="settings.xCookieBusy"
                  :disabled="!settings.xCookieDraft.trim()"
                  @click="settings.saveXCredentials()"
                >
                  Kaydet
                </RvButton>
                <RvButton
                  variant="secondary"
                  size="sm"
                  :loading="settings.xCookieBusy"
                  :disabled="!settings.xCookieSaved"
                  @click="settings.checkXCredentials()"
                >
                  Bağlantıyı test et
                </RvButton>
                <RvButton
                  v-if="settings.xCookieSaved"
                  variant="ghost"
                  size="sm"
                  @click="settings.removeXCredentials()"
                >
                  Sil
                </RvButton>
              </div>

              <span v-if="settings.xStatusText" class="key-status" :class="settings.xStatus">
                <Check v-if="settings.xStatus === 'ok'" :size="12" />
                <CircleAlert v-else-if="settings.xStatus === 'error'" :size="12" />
                {{ settings.xStatusText }}
              </span>

              <span class="note">
                x.com'da oturum açıkken Cookie-Editor eklentisiyle çerezleri dışa aktar ve
                buraya yapıştır — JSON çıktısı da, <span class="rv-mono">Cookie</span> başlığı
                da olur. Yalnızca <span class="rv-mono">auth_token</span> ve
                <span class="rv-mono">ct0</span> alınır, gerisi atılır. İkisi
                <strong>aynı oturumdan, aynı anda</strong> alınmalı; eşleşmezlerse X isteği
                reddediyor.
              </span>

              <div v-if="!settings.xToolInstalled" class="access-state none">
                <TriangleAlert :size="14" class="access-icon" />
                <span>
                  X içeriği <span class="rv-mono">bird</span> aracıyla çekiliyor ve bu araç
                  kurulu değil. Kurmak için:
                  <span class="rv-mono">npm install -g @steipete/bird</span>
                </span>
              </div>

              <div class="privacy">
                <Lock :size="14" class="privacy-icon" />
                Çerezler sistem anahtar zincirine yazılır, arayüze bir daha okunmaz ve yalnızca
                X'e giden isteklerde kullanılır. Hesabına tam erişim verdiği için kullanmadığında
                silmen iyi olur.
              </div>
            </div>
          </div>
        </section>

        <section class="divided">
          <span class="section-label">Uygulama</span>

          <div class="row">
            <span class="row-label">Görünüm</span>
            <RvSegmented
              :model-value="settings.theme"
              :options="[
                { value: 'dark', label: 'Koyu' },
                { value: 'light', label: 'Açık' },
              ]"
              @update:model-value="settings.theme = $event as 'dark' | 'light'"
            />
          </div>

          <div class="row">
            <span class="row-label">Açılışta göster</span>
            <RvSelect
              :model-value="settings.general.startOnLaunch"
              :options="[
                { value: 'new', label: 'Yeni Video' },
                { value: 'library', label: 'Kitaplık' },
                { value: 'setup', label: 'Kurulum denetimi' },
              ]"
              class="control"
              @update:model-value="
                settings.general.startOnLaunch = $event as 'new' | 'library' | 'setup'
              "
            />
          </div>

          <div class="row">
            <span class="row-label">Bitince bildir</span>
            <RvSwitch v-model="settings.general.notifyOnFinish" />
            <span class="note">Video hazır olduğunda bildirim ve kısa bir ses</span>
          </div>

          <div class="row">
            <span class="row-label">Üstte tut</span>
            <RvSwitch v-model="settings.general.keepWindowOnTop" />
            <span v-if="settings.windowError" class="key-status error">
              <CircleAlert :size="12" />
              {{ settings.windowError }}
            </span>
            <span v-else class="note">Pencere diğer uygulamaların üstünde kalsın</span>
          </div>
        </section>
      </template>

      <!-- SESLENDİRME -->
      <template v-else-if="active === 'tts'">
        <header>
          <h1>Seslendirme</h1>
          <p>Varsayılan motor ve ses, tüm yeni videolarda kullanılır.</p>
        </header>

        <section>
          <span class="section-label">API anahtarları</span>

          <div v-for="key in settings.apiKeys" :key="key.id" class="key-block">
            <label>{{ key.label }}</label>
            <div class="key-row">
              <RvTextField
                v-model="key.draft"
                type="password"
                mono
                class="grow"
                :placeholder="
                  key.saved
                    ? 'Kayıtlı — değiştirmek için yeni anahtarı yapıştır'
                    : 'Anahtarı yapıştır'
                "
              />
              <RvButton
                variant="secondary"
                size="sm"
                :loading="key.busy"
                :disabled="!key.draft.trim()"
                @click="settings.saveEngineKey(key.id)"
              >
                Kaydet
              </RvButton>
              <RvButton
                variant="secondary"
                size="sm"
                :loading="key.busy"
                :disabled="!key.saved"
                @click="settings.checkEngine(key.id)"
              >
                Bağlantıyı test et
              </RvButton>
              <RvButton
                v-if="key.saved"
                variant="ghost"
                size="sm"
                @click="settings.removeEngineKey(key.id)"
              >
                Sil
              </RvButton>
            </div>

            <span v-if="key.statusText" class="key-status" :class="key.status">
              <Check v-if="key.status === 'ok'" :size="12" />
              <CircleAlert v-else-if="key.status === 'error'" :size="12" />
              {{ key.statusText }}
            </span>
            <span v-else-if="key.saved" class="key-status">
              Anahtar kayıtlı — henüz test edilmedi.
            </span>
          </div>

          <div class="privacy">
            <Lock :size="14" class="privacy-icon" />
            Anahtarlar yalnızca bu bilgisayarda, sistem anahtar zincirinde saklanır; kaydedildikten
            sonra arayüze bir daha okunmaz ve hiçbir sunucuya gönderilmez.
          </div>

          <div class="note">
            Google Translate ve sistem sesi anahtar istemez; ikisi de anahtarsız çalışır.
          </div>
        </section>

        <section class="divided">
          <span class="section-label">Varsayılanlar</span>

          <div class="row">
            <span class="row-label wide">Varsayılan motor</span>
            <RvSelect
              v-model="settings.defaults.engineId"
              :options="engineOptions"
              class="control"
            />
          </div>

          <div class="row">
            <span class="row-label wide">Varsayılan ses</span>
            <RvSelect
              v-model="settings.defaults.voiceId"
              :options="voiceOptions"
              class="control"
            />
          </div>

          <div class="row">
            <span class="row-label wide">Kota bitince yedek motor</span>
            <RvSwitch v-model="settings.defaults.fallbackOnQuota" />
            <span class="note">Hata alınca Google Translate'e düş</span>
          </div>
        </section>
      </template>

      <!-- ÇEVİRİ -->
      <template v-else-if="active === 'ceviri'">
        <header>
          <h1>Çeviri</h1>
          <p>Yorumlar seslendirilmeden önce bu ayarlarla çevrilir.</p>
        </header>

        <section>
          <div class="row">
            <span class="row-label wide">Çeviri servisi</span>
            <RvSelect
              v-model="settings.translation.provider"
              :options="[
                { value: 'google', label: 'Google Translate' },
                { value: 'none', label: 'Çevirme (özgün dilde bırak)' },
              ]"
              class="control"
            />
          </div>

          <div class="row">
            <span class="row-label wide">Hedef dil</span>
            <RvSelect
              v-model="settings.translation.targetLang"
              :options="[
                { value: 'tr', label: 'Türkçe' },
                { value: 'en', label: 'İngilizce' },
                { value: 'de', label: 'Almanca' },
                { value: 'es', label: 'İspanyolca' },
              ]"
              class="control"
            />
          </div>

          <div class="row">
            <span class="row-label wide">Zaten hedef dildeyse atla</span>
            <RvSwitch v-model="settings.translation.skipIfSameLanguage" />
            <span class="note">Türkçe gönderiyi tekrar çevirip bozmaz</span>
          </div>

          <div class="row top">
            <span class="row-label wide">Sözlük</span>
            <div class="stack">
              <textarea
                v-model="settings.translation.glossary"
                class="textarea rv-mono"
                rows="4"
              ></textarea>
              <span class="note">
                Her satıra bir kural: <span class="rv-mono">kaynak=hedef</span>. Çeviri
                bittikten sonra uygulanır.
              </span>
            </div>
          </div>
        </section>
      </template>

      <!-- VİDEO -->
      <template v-else-if="active === 'video'">
        <header>
          <h1>Video</h1>
          <p>Çıktı biçimi ve dosya yönetimi.</p>
        </header>

        <section>
          <div class="row">
            <span class="row-label wide">Çözünürlük</span>
            <RvSelect
              v-model="settings.video.resolution"
              :options="[
                { value: '1080x1920', label: '1080 × 1920 (9:16)' },
                { value: '720x1280', label: '720 × 1280 (9:16)' },
                { value: '1440x2560', label: '1440 × 2560 (9:16)' },
              ]"
              class="control"
            />
          </div>

          <div class="row">
            <span class="row-label wide">Kare hızı</span>
            <RvSegmented
              v-model="settings.video.fps"
              :options="[
                { value: '24', label: '24' },
                { value: '30', label: '30' },
                { value: '60', label: '60' },
              ]"
            />
          </div>

          <div class="row">
            <span class="row-label wide">Kodek</span>
            <RvSelect
              v-model="settings.video.codec"
              :options="[
                { value: 'h264', label: 'H.264 · yuv420p (en uyumlu)' },
                { value: 'h265', label: 'H.265 · daha küçük dosya' },
              ]"
              class="control"
            />
          </div>

          <div class="row">
            <span class="row-label wide">Bit hızı</span>
            <div class="control slider-wrap">
              <RvSlider v-model="settings.video.bitrateMbps" :min="2" :max="20" />
              <span class="rv-tabular note">{{ settings.video.bitrateMbps }} Mbps</span>
            </div>
          </div>

          <div class="row">
            <span class="row-label wide">Kapanış payı</span>
            <div class="control slider-wrap">
              <RvSlider v-model="settings.video.outroSec" :min="0" :max="10" :step="1" />
              <span class="rv-tabular note">{{ settings.video.outroSec }} sn</span>
            </div>
            <span class="note">Son karttan sonra arka plan yalnız devam eder</span>
          </div>

          <div class="row">
            <span class="row-label wide">Donanım hızlandırma</span>
            <RvSwitch v-model="settings.video.hardwareAccel" />
            <span class="note">VideoToolbox — Apple silicon'da belirgin hızlanma</span>
          </div>
        </section>

        <section class="divided">
          <span class="section-label">Klasörler</span>

          <div class="row">
            <span class="row-label wide">Çıktı klasörü</span>
            <button
              type="button"
              class="path-field clickable"
              title="Klasörde göster"
              @click="openFolder(settings.video.outputDir)"
            >
              <Folder :size="14" />
              {{ settings.video.outputDir }}
            </button>
            <RvButton variant="secondary" size="sm" @click="pickFolder('output')">
              Değiştir
            </RvButton>
          </div>

          <div class="row">
            <span class="row-label wide">Arka plan klasörü</span>
            <button
              type="button"
              class="path-field clickable"
              title="Klasörde göster"
              @click="openFolder(settings.backgroundsDir)"
            >
              <Folder :size="14" />
              {{ settings.backgroundsDir || "Seçilmedi — yalnızca uygulamayla gelenler" }}
            </button>
            <RvButton variant="secondary" size="sm" @click="pickFolder('backgrounds')">
              Değiştir
            </RvButton>
          </div>

          <div class="row">
            <span class="row-label wide">Müzik klasörü</span>
            <button
              type="button"
              class="path-field clickable"
              title="Klasörde göster"
              @click="openFolder(settings.musicDir)"
            >
              <Folder :size="14" />
              {{ settings.musicDir || "Seçilmedi — yalnızca uygulamayla gelenler" }}
            </button>
            <RvButton variant="secondary" size="sm" @click="pickFolder('music')">
              Değiştir
            </RvButton>
          </div>

          <div class="row top">
            <span class="row-label wide">Dosya adı şablonu</span>
            <div class="stack">
              <RvTextField v-model="settings.video.nameTemplate" mono />
              <span class="note rv-mono">{{ fileNameSample }}</span>
              <span class="note">
                Kullanılabilir alanlar:
                <span class="rv-mono">{tarih}</span>,
                <span class="rv-mono">{subreddit}</span>,
                <span class="rv-mono">{baslik-kisa}</span>
              </span>
            </div>
          </div>
        </section>

        <section class="maintenance">
          <div class="maint-head">
            <span class="strong">Bakım</span>
            <span class="note">
              {{ settings.maintenanceSummary }}
            </span>
          </div>
          <div class="maint-actions">
            <RvButton
              variant="secondary"
              size="sm"
              :loading="settings.maintenanceBusy"
              @click="settings.clearTempFiles()"
            >
              Geçici dosyaları temizle
            </RvButton>
            <RvButton
              variant="ghost"
              size="sm"
              :loading="settings.maintenanceBusy"
              @click="settings.clearThumbnailCache()"
            >
              Kapak önbelleğini boşalt
            </RvButton>
            <RvButton
              variant="danger"
              size="sm"
              class="push"
              :loading="settings.maintenanceBusy"
              @click="settings.deleteAllOutputs()"
            >
              Tüm çıktıları sil
            </RvButton>
          </div>
          <span v-if="settings.maintenanceMessage" class="key-status ok">
            {{ settings.maintenanceMessage }}
          </span>
        </section>
      </template>

      <!-- GELİŞMİŞ -->
      <template v-else>
        <header>
          <h1>Gelişmiş</h1>
          <p>Çalışma ortamı ve hata ayıklama.</p>
        </header>

        <section>
          <div class="row">
            <span class="row-label wide">Ağ zaman aşımı</span>
            <div class="control slider-wrap">
              <RvSlider v-model="settings.advanced.timeoutSec" :min="10" :max="120" :step="5" />
              <span class="rv-tabular note">{{ settings.advanced.timeoutSec }} sn</span>
            </div>
          </div>

          <div class="row">
            <span class="row-label wide">Ortam denetimi</span>
            <RvButton variant="secondary" size="sm" @click="router.push('/kurulum')">
              Kurulum ekranını aç
            </RvButton>
            <span class="note">{{ settings.environmentSummary }}</span>
          </div>

          <div class="row top">
            <span class="row-label wide">Çalışma klasörü</span>
            <div class="stack">
              <button
                type="button"
                class="path-field clickable"
                title="Klasörde göster"
                @click="openFolder(settings.tempDir)"
              >
                <Folder :size="14" />
                {{ settings.tempDir }}
              </button>
              <span class="note">
                Ses parçaları, kapak görselleri ve ara dosyalar burada tutulur.
              </span>
            </div>
          </div>
        </section>
      </template>
    </main>
  </div>
</template>

<style scoped>
.wrap {
  flex: 1;
  min-width: 0;
  display: flex;
  min-height: 0;
}

/* Sekme rayı */
.rail {
  width: 184px;
  flex: none;
  border-right: 1px solid var(--rv-border);
  padding: var(--rv-space-4) 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.rail-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--rv-text-faint);
  letter-spacing: 0.02em;
  text-transform: uppercase;
  padding: 0 var(--rv-space-2) 10px;
}

.tab {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  height: 30px;
  padding: 0 var(--rv-space-2);
  border-radius: var(--rv-radius-sm);
  background: transparent;
  border: 1px solid transparent;
  color: var(--rv-text-muted);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
  text-align: left;
}

.tab:hover:not(.on) {
  color: var(--rv-text);
}

.tab.on {
  background: var(--rv-bg-elevated);
  border-color: var(--rv-border);
  color: var(--rv-text);
  font-weight: 500;
}

.tab.on .tab-icon {
  color: var(--rv-accent-quiet);
}

/* Panel */
.panel {
  flex: 1;
  min-width: 0;
  padding: 28px 32px;
  display: flex;
  flex-direction: column;
  gap: 26px;
  max-width: 760px;
}

header {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-1);
}

h1 {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
}

header p {
  margin: 0;
  color: var(--rv-text-muted);
}

section {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

section.divided {
  padding-top: 20px;
  border-top: 1px solid var(--rv-border);
}

.section-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--rv-text-faint);
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

.row {
  display: flex;
  align-items: center;
  gap: var(--rv-space-4);
}

.row.top {
  align-items: flex-start;
}

.row-label {
  color: var(--rv-text-muted);
  width: 120px;
  flex: none;
}

.row-label.wide {
  width: 170px;
}

.row.top .row-label {
  padding-top: 8px;
}

.control {
  flex: 1;
  max-width: 300px;
}

.grow {
  flex: 1;
  min-width: 0;
}

.stack {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-width: 460px;
}

.note {
  font-size: 12px;
  color: var(--rv-text-faint);
  line-height: 1.5;
}

.slider-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: 280px;
}

/* Reddit erişim durumu */
.access-state {
  display: flex;
  align-items: flex-start;
  gap: var(--rv-space-2);
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  font-size: 12px;
  line-height: 1.5;
  color: var(--rv-text-muted);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
}

.access-state.none {
  background: var(--rv-warning-soft);
  border-color: color-mix(in srgb, var(--rv-warning) 28%, transparent);
}

.access-icon {
  flex: none;
  margin-top: 1px;
  color: var(--rv-text-faint);
}

.access-state.clientId .access-icon,
.access-state.cookie .access-icon {
  color: var(--rv-success);
}

.access-state.none .access-icon {
  color: var(--rv-warning);
}

.ext-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  align-self: flex-start;
}

/* Anahtarlar */
.key-block {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.key-block label {
  font-size: 12px;
  color: var(--rv-text-muted);
}

.key-row {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
}

.key-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.key-status.ok {
  color: var(--rv-success);
}

.key-status.error {
  color: var(--rv-danger);
}

.privacy {
  display: flex;
  align-items: flex-start;
  gap: var(--rv-space-2);
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.55;
}

.privacy-icon {
  color: var(--rv-text-faint);
  flex: none;
  margin-top: 2px;
}

.textarea {
  padding: var(--rv-space-2) 10px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
  color: var(--rv-text);
  font-size: 12px;
  line-height: 1.6;
  resize: vertical;
  outline: none;
}

.textarea:focus {
  border-color: var(--rv-accent);
  box-shadow: var(--rv-focus);
}

.path-field {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  height: 32px;
  padding: 0 10px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
  font-family: var(--rv-font-mono);
  font-size: 12px;
  color: var(--rv-text-muted);
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.clickable {
  cursor: pointer;
}

.clickable:hover {
  border-color: var(--rv-border-strong);
  color: var(--rv-text);
}

/* Bakım */
.maintenance {
  gap: 10px;
  padding: 14px;
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
}

.maint-head {
  display: flex;
  align-items: center;
}

.strong {
  font-weight: 500;
}

.maint-head .note {
  margin-left: auto;
}

.maint-actions {
  display: flex;
  gap: var(--rv-space-2);
}

.push {
  margin-left: auto;
}
</style>
