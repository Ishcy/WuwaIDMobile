# 04 — UI dan lokalisasi

## Korpus dan implikasi i18n

Tidak ada resource string milik aplikasi: resource Android yang ada berasal dari
AndroidX; copy WAHU di-hardcode sebagai literal Kotlin. Seluruh 393 string
Vietnam tersedia di [`strings/vi-strings.json`](strings/vi-strings.json)
(berkelompok) dan [`strings/vi-strings.tsv`](strings/vi-strings.tsv)
(`group` / `class` / `text`).

| Grup | Jumlah | Catatan port |
|---|---:|---|
| `ui` | 134 | Teks layar |
| `controller` | 13 | Status/aksi controller |
| `core` | 2 | Label varian |
| `installer` | 244 | Mayoritas log/error internal; tidak semuanya perlu diterjemahkan |

Bangun i18n dari nol untuk UI Indonesia. Terjemahkan copy user-facing, termasuk
label/kategori/sample font dari `assets/font/catalog.json`; pesan internal
installer dapat tetap diagnostik.

## Layar dan perilaku

### Root dan navigasi

Header: `WAHU Mobile 0.2.0-alpha.9 · 3.7.0-R2 · không ghi đè PAK gốc.`
Subjudul: `Chọn bản dịch, cài patch rồi mở game.`

Bottom bar `MobileSection`:

| Enum | Label | Sublabel |
|---|---|---|
| `HOME` | `Trang chủ` | `Cài nhanh · setup · mở game` |
| `CUSTOMIZE` | `Tùy biến` | `Font và hiển thị` |
| `TOOLS` | `Bảo trì` | `Kiểm tra · repair · chẩn đoán` |

Badge kesiapan dari `MobileReadiness`:

| Readiness | Badge |
|---|---|
| `INITIALIZING` | `ĐANG KIỂM TRA` |
| `READY` | `ĐÃ CÀI` / `SẴN SÀNG` |
| `GAME_MISSING` | `CHƯA CÓ GAME` |
| `GAME_VERSION_UNSUPPORTED` | `CHƯA HỖ TRỢ` |
| `RESOURCE_NOT_READY` | `THIẾU DỮ LIỆU` |
| `RESOURCE_VERSION_UNSUPPORTED` | `RESOURCE LỆCH` |
| `SHIZUKU_NOT_RUNNING` | `CẦN SHIZUKU` |
| `SHIZUKU_PERMISSION_REQUIRED` | `CẦN QUYỀN` |

`MobileDashboardLogic` juga membangun label kompatibilitas, hint dan aksi utama
dari readiness; aksi hanya aktif saat prasyaratnya siap. Label/detail dan seluruh
variasi copy tercatat di korpus.

### HOME — instalasi

Varian pilihan dan status file:

| Label | Enum | Payload |
|---|---|---|
| `Tên Anh` | `ENGLISH_NAMES` | `WuWaVH_EN_99_P.pak` |
| `Hán Việt` | `HAN_VIET` | `WuWaVH_HanViet_99_P.pak` |

Status varian: belum dipasang (`Chưa cài`), setup dibutuhkan (`Cần setup`), tidak
kompatibel (`Chưa tương thích`), atau perlu instal ulang (`Cài lại`). Aksi utama
mencakup `Cài Việt hóa`, `Cài lại`, `Cài nhanh`, dan `Cài Việt hóa & mở game`;
instalasi menunggu Game, Shizuku, dan Resource kompatibel.

Tombol korektif mengikuti `MobilePrimaryAction`:

| Kondisi | Label | Aksi |
|---|---|---|
| Game tidak ditemukan | `Tải Wuthering Waves` | `OPEN_GAME_STORE` |
| Shizuku berhenti | `Thiết lập Shizuku` | `OPEN_SHIZUKU` |
| Permission belum diberikan | `Cấp quyền Shizuku` | `REQUEST_SHIZUKU_PERMISSION` |
| Siap | `Cài Việt hóa` / `Mở game` | `INSTALL_AND_OPEN` / `OPEN_GAME` |

Pesan kesalahan, progress, status perangkat, dan instruksi setup lengkap ada di
korpus; jangan menyalin daftar pesan ke dokumen ini.

### CUSTOMIZE — font

Impor TTF/OTF/TTC. Font yang tidak memenuhi gate glyph hanya dapat dipreview;
batas ukuran impor adalah `Font quá lớn (>30MB)`. Gate APK khusus Vietnam:
`FontFileCodec` mensyaratkan 134 codepoint (67 huruf kecil + uppercase via
`Locale.ROOT`), sesuai `viTotal: 134` pada katalog.

Port Indonesia tidak boleh memakai gate Vietnam apa adanya: alfabetnya memiliki
tanda diakritik lain dan tidak memakai tanda nada Vietnam. Jadikan glyph wajib
sebagai data per-locale, lalu simpan jumlah yang lolos per locale (bukan
`viHave`/`viTotal`). Label serta dialog pratinjau/impor tersedia di korpus.

### TOOLS — perawatan

UI menyediakan pemeriksaan sistem, pemeriksaan ulang, repair, ekspor laporan
diagnostik, dan penghapusan patch. Dialog penghapusan menyatakan bahwa hanya file
patch yang dikelola WAHU dihapus; file game asli tidak ditimpa. Tampilkan aksi
yang sesuai dengan status saat ini. Detail dan copy lengkap ada di korpus.

## Panduan port Bahasa Indonesia

- Buat katalog teks/i18n sendiri; APK tidak menyediakan resource string yang
  dapat diterjemahkan.
- Bila menyediakan varian payload Indonesia, definisikan enum dan filename
  target sendiri (contoh usulan: `INDONESIAN` / `WuWaVH_ID_99_P.pak`); jangan
  menganggap contoh itu berasal dari APK.
- Lokalkan kategori dan sample font. Ganti gate `viHave`/`viTotal` dengan
  codepoint wajib per locale.
- Pertahankan istilah Shizuku, Repair, PAK, dan resource; contoh terminologi:
  `Việt hóa` → `Bahasa Indonesia`, `Trang chủ` → `Beranda`,
  `Tùy biến` → `Kustomisasi`, `Bảo trì` → `Perawatan`, `Tên Anh` → `Nama Inggris`.
  Pertahankan `Hán Việt` hanya jika varian itu tetap disediakan.
- Ubah kalimat yang dirakit dengan concatenation menjadi template placeholder
  (mis. ICU), bukan menerjemahkan fragmennya satu per satu.