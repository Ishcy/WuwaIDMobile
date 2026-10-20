# 08 — Discovery, Compatibility, Updater & Design System

## 1. Resource version discovery

`storage/ResourceVersionDiscoverer` — one method, two fields:

```kotlin
class ResourceVersionDiscoverer {
    private val semanticVersion = Regex("^\d+\.\d+\.\d+$")
    private val hallmarks = listOf("Mount", "Resource", "Launcher", "Lang_en")

    suspend fun discover(storage: SafeGameStorage): ResourceVersionDiscoveryResult
}
```

### Candidate scan and ordering

`discover()` checks `storage.capability()` (unreadable → `blocker = "Storage root is not readable"`), lists `Resources/`, and retains directory names matching `^\d+\.\d+\.\d+$`. For each it stats `Mount`, `Resource`, `Launcher`, `Lang_en`, derives `hasMountRegistry` from the `Mount` registry directory, and builds `Candidate(version, parts, hasMountRegistry, hallmarkCount)`.

Sort descending by `hasMountRegistry`, `hallmarkCount`, then `parts` three times (same natural `List<Int>` ordering). `parts` is numeric (`Candidate` signature `(Ljava/lang/String;Ljava/util/List;ZI)V`, generic element `Ljava/lang/Integer;`), so `3.10.0 > 3.9.0`. Mount presence and hallmark count outrank version: a lower version with a mount registry can beat a newer one without. `ResourceVersionDiscoverer` does not reference `core.SemanticVersion` (used by `CompatibilityRules` and `ClientCompatibility`).

Return `ResourceVersionDiscoveryResult(selected: String?, candidates: List<Candidate>, blocker: String?)`; no candidates → `"No materialized resource-version directory was found"`. Do not use the hard-coded `3.7.0` fallback string as the discovery rule.

---

## 2. Compatibility rules

`core/CompatibilityRules.evaluate(game, manifest, variant)`. One method, 13 string constants,
traced in source order:

| Order | Check | Message |
|---|---|---|
| 1 | game installed | `Target game is not installed` |
| 2 | `game.packageName == "com.kurogame.wutheringwaves.global"` | `Unexpected package: <p>` |
| 3 | `game.versionName` present | `Game version is missing` |
| 4 | version matches semver | `Malformed game version: <v>` |
| 5 | resource version present | `Resource version is missing` |
| 6 | resource version matches semver | `Malformed resource version: <v>` |
| 7 | manifest has an artifact for the variant | `Variant artifact is missing` / `<variant> is not in the manifest` |
| 8 | game version matches a compatibility entry | `Game version <v> …` |
| 9 | resource version matches that entry | `Resource version <v> …` |
| 10 | `game.versionCode` present | `Game versionCode is missing` |
| 11 | versionCode matches that entry | `Game versionCode <c> …` |
| — | success | `Compatible with <version>/<n>` |

Game version, resource version, and `versionCode` must match the **same** compatibility entry. Messages are English; the UI copy is Vietnamese.

---

## 3. Updater signature verification

`ManifestSignatureVerifier.verify(payload: ByteArray, signature: DetachedManifestSignature): Boolean`;
implementation `Ed25519ManifestVerifier(trustedKeys: Map<String, Key>)`.
`SignedManifestLoader.verifyAndParse(bytes: ByteArray, signature): UpdaterManifest`.
`DetachedManifestSignature(keyId: String, signatureBase64: String)`.

| Validation | Failure |
|---|---|
| `keyId` matches `^[A-Za-z0-9._-]{1,64}$` | `Invalid keyId` |
| `signatureBase64` non-empty | `Manifest signature is empty` |
| Every trusted-key ID matches the same regex | `Invalid trusted manifest key id` |
| `trustedKeys` non-empty | `At least one trusted manifest public key is required` |
| Ed25519 verification fails | `Updater manifest signature verification failed` |

The detached signature covers raw `payload` bytes, not the wrapper; `keyId` selects the
trusted public key. The updater is unwired in this APK (no `SignedManifestLoader` call site
or base URL), so no release-selection order is established. The trusted-key map is the
whole trust root; key distribution remains a port decision.
---

## 4. Design system

`res/values/colors.xml` and `res/values/styles.xml` each contain one app resource; the palette is Kotlin.

### 4.1 Android-level (the only resource-level design surface)

```xml
<style name="Theme.VHWuWa" parent="@android:style/Theme.Material.NoActionBar">
    <item name="android:windowBackground">#ff0b1417</item>
    <item name="android:fontFamily">sans</item>
    <item name="android:statusBarColor">#ff0b1417</item>
    <item name="android:navigationBarColor">#ff0b1417</item>
    <item name="android:windowLightStatusBar">false</item>
    <item name="android:windowLightNavigationBar">false</item>
</style>
<color name="wahu_launcher_background">#ff071923</color>
```

Dark-only; light status/navigation bar icons are disabled. No light theme or dynamic-colour path.

### 4.2 Compose tokens — `PremiumInstallerUiKt`, 14 named colours

Extracted from `<clinit>` (`const-wide` → `ColorKt.Color(J)` → `sput-wide`):

| Token | ARGB | Role |
|---|---|---|
| `WahuCanvas` | `#FF0B1417` | app background (matches `Theme.VHWuWa`) |
| `WahuPanelColor` | `#FF111A1F` | card / panel surface |
| `WahuLine` | `#FF2A373B` | borders and dividers |
| `WahuInk` | `#FFF2EFE6` | primary text (warm off-white) |
| `WahuMuted` | `#FFA4ADB0` | secondary text |
| `WahuDisabled` | `#FF20292D` | disabled surface |
| `WahuDisabledText` | `#FF697579` | disabled label |
| `WahuMint` | `#FFD0C19B` | primary accent (warm sand) |
| `WahuMintStrong` | `#FFE6DAC0` | accent hover / emphasis |
| `WahuMintSoft` | `#FF25271F` | accent-tinted surface |
| `WahuBlueSoft` | `#FF101C23` | info-tinted surface |
| `WahuWarningSoft` | `#FF2A2317` | warning-tinted surface |
| `WahuDangerSoft` | `#FF32191F` | danger-tinted surface |
| `WahuDanger` | `#FFE85C70` | danger accent |

The palette is a **cool near-black teal** (`#0B1417`) with a **warm sand accent** (`#D0C19B`) —
an intentionally non-standard combination, not Material You.

### 4.3 Colour scheme

`FullFeatureMobileScreenKt` calls Material 3 `darkColorScheme(...)` with default values: all
long arguments are `const-wide 0x0` except mask `0xffff` and marker `0x0`. Visible color comes
from per-component `Wahu*` tokens, not `MaterialTheme.colorScheme` (unused scaffolding).

Adjacent `const v97, -0x407e224` reads ARGB `#FBF81DDC`, but its slot cannot be attributed:
the argument range/mask do not match the 35-long signature. It is excluded from the 14-token
set; `extract_design_tokens.py` is the reproducible source for those tokens.

### 4.4 Inline literals beyond the tokens

The two screen files also hardcode 17 non-token ARGB values: `#FF182329`, `#FF11171A`,
`#FF0F181C`, `#FF27343A`, `#FF304049`, `#FF465459`, `#FF746B55`, `#FF7A3F4A`,
`#FF879296`, `#FFB5C7D0`, `#FFD5BC83`, `#FFE0C27A`, `#FFE1BC70`, `#FFE2A8B3`,
`#FFF0A1AF`, `#FF19262B`, `#FF182126`; lift them from composables for a pixel-faithful port.

### 4.5 Tone system

```kotlin
enum class UiTone { DANGER, GOOD, INFO, NEUTRAL, WARNING }   // alphabetical declaration
```

`FullFeatureMobileScreenKt.readinessTone(readiness, busy)` maps readiness to `UiTone`, rendered by `InlineNotice`. The mapping uses a compiled `tableswitch` on `ordinal()`; the exact table remains unknown and requires `pswitch_data_0`.

### 4.6 Typography and shapes — not extracted

Window `android:fontFamily` is `sans`; body `TextStyle`/`Typography` and `Shapes` declarations were not located. The 38 `res/drawable*`/`res/mipmap*` entries are standard launcher/icons. Typography and shape scale remain undefined; re-derive from runtime screenshots if pixel fidelity matters.
