# Tool dan reproduksi

Reproduksi hasil memakai APK sumber bernama
`reverse_engineer/VHWuWa-Android-0.2.0-alpha.9-3.7.0-R2.apk` (third-party,
gitignored); sediakan file itu secara terpisah.

| Tool | Versi / lokasi |
|---|---|
| `apktool` | 2.11.1, `~/.local/re-tools/apktool.jar` |
| `jadx` | 1.5.1, `~/.local/re-tools/jadx/` |
| `aapt2` | 35.0.0, `/opt/android-sdk/build-tools/35.0.0/` |
| `python3` | Sistem; skrip dalam `reverse_engineer/tools/` |

Pasang paket apktool/jadx:

```bash
mkdir -p ~/.local/re-tools && cd ~/.local/re-tools
curl -sLO https://github.com/iBotPeaches/Apktool/releases/download/v2.11.1/apktool_2.11.1.jar
curl -sLO https://github.com/skylot/jadx/releases/download/v1.5.1/jadx-1.5.1.zip
unzip -q jadx-1.5.1.zip -d jadx
```

Wrapper repo memakai Java:

- `tools/apktool.sh` → `java -Xmx4g -jar ~/.local/re-tools/apktool.jar "$@"`
- `tools/jadx.sh` → `java -Xmx8g -jar ~/.local/re-tools/jadx/lib/jadx-1.5.1-all.jar "$@"`
- `tools/summ.py` → ringkasan class smali (fields, methods, const-string pool)
- `tools/verify_charset.py` → gate glyph 67 + 67 = 134
- `tools/extract_design_tokens.py` → 14 token `Wahu*` + 17 literal inline
- `tools/check_docs.py` → cek aksara asing/kata korup/baris kembar/fence dan blok JSON

Semua helper ada di `reverse_engineer/tools/`; hasil analisis ada di
`reverse_engineer/out/` dan di-gitignore. Hasil dokumentasi diturunkan dari
smali; jadx opsional dan bukan sumber temuan. Jika dipakai, batasi ke dex app
`classes2`–`classes7` seperti perintah di bawah, bukan seluruh APK (`classes.dex`
sendiri berisi 17.865 class Compose/AndroidX/Kotlin).

## Reproduksi APK

Jalankan dari root repo; `OUT` tetap di direktori gitignored `reverse_engineer/out/`.

```bash
cd <repo>
OUT=reverse_engineer/out

# Unpack APK mentah
mkdir -p "$OUT/raw"
unzip -o -q reverse_engineer/VHWuWa-Android-0.2.0-alpha.9-3.7.0-R2.apk -d "$OUT/raw"

# Identitas dan decode smali/resource/manifest
aapt2 dump badging reverse_engineer/VHWuWa-Android-0.2.0-alpha.9-3.7.0-R2.apk
reverse_engineer/tools/apktool.sh d -f -o "$OUT/apktool" \
  reverse_engineer/VHWuWa-Android-0.2.0-alpha.9-3.7.0-R2.apk

# Opsional: decompile hanya dex app, bukan seluruh APK
reverse_engineer/tools/jadx.sh -d "$OUT/jadx" --show-bad-code --threads-count 4 \
  "$OUT"/raw/classes{2,3,4,5,6,7}.dex

# Hash payload untuk dibandingkan dengan core.ContentBaseline
sha256sum "$OUT"/raw/assets/payload/*.pak
```

## Distribusi class smali

```bash
for d in reverse_engineer/out/apktool/smali reverse_engineer/out/apktool/smali_classes*; do
  printf '%-42s total=%-6s vn.wahu=%s\n' "$d" \
    "$(find "$d" -name '*.smali' | wc -l)" \
    "$(find "$d" -path '*vn/wahu*' -name '*.smali' | wc -l)"
done
```

## Ringkasan satu class

```bash
python3 reverse_engineer/tools/summ.py core.CompatibilityRules
python3 reverse_engineer/tools/summ.py installer.AndroidPatchMount
python3 reverse_engineer/tools/summ.py storage.ShizukuStorageAdapter
python3 reverse_engineer/tools/summ.py storage.StoragePathGuard
```

Argumen bisa `pkg.Class` (mis. `core.Models`) atau nama direktori package
(mis. `installer`) untuk merangkum semua class di dalamnya.

## Ekstraksi string Vietnam

Semua string app adalah `const-string` di smali. Skrip yang dipakai:

```bash
python3 - <<'PY'
import re, glob, os, json
from collections import defaultdict
VI = set("àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđ")
VI |= {c.upper() for c in VI}
pat = re.compile(r'const-string(?:/[jA-Za-z]*)?\s+\S+,\s+"((?:[^"\\]|\\.)*)"')
ESC = {'n': '\n', 't': '\t', '"': '"', '\\': '\\', "'": "'",
       'b': '\b', 'r': '\r', 'f': '\f', '0': '\0'}
def unesc(s):                      # WAJIB: smali escaped non-ASCII sebagai \uXXXX
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c == '\\' and i + 1 < len(s):
            if s[i+1] == 'u' and i + 5 < len(s):
                try:
                    out.append(chr(int(s[i+2:i+6], 16))); i += 6; continue
                except ValueError:
                    pass
            out.append(ESC.get(s[i+1], s[i+1])); i += 2; continue
        out.append(c); i += 1
    return ''.join(out)
groups, seen = defaultdict(list), set()
for p in sorted(glob.glob("reverse_engineer/out/apktool/smali*/vn/wahu/vhwuwa/**/*.smali", recursive=True)):
    pkg = os.path.basename(os.path.dirname(p)); cls = os.path.basename(p)[:-6]
    top = cls.split('$')[0]
    UI = ('FullFeatureMobileScreenKt', 'MobileDashboardLogic', 'MobileDashboardState',
          'MobileSection', 'MobilePrimaryAction', 'MobileReadiness', 'MobileUiPresentation',
          'PremiumInstallerUiKt', 'FontGalleryPanelKt', 'FontCatalogRepository',
          'FontCatalogRepositoryKt', 'FontCatalogItem', 'UiTone', 'WahuButtonKind')
    group = pkg if pkg != 'vhwuwa' else ('ui' if top in UI else 'controller')
    for line in open(p, encoding="utf-8", errors="replace"):
        m = pat.search(line)
        if not m: continue
        s = unesc(m.group(1))
        if not any(ch in VI for ch in s) or len(s) < 2 or ' ' not in s: continue
        if '\n' in s or s in seen: continue
        seen.add(s); groups[group].append({"text": s, "class": f"{pkg}.{cls}"})
out = {"source": "VHWuWa-Android-0.2.0-alpha.9-3.7.0-R2.apk",
       "count": sum(len(v) for v in groups.values()),
       "groups": {k: groups[k] for k in sorted(groups)}}
with open("reverse_engineer/strings/vi-strings.json", "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=1)
with open("reverse_engineer/strings/vi-strings.tsv", "w", encoding="utf-8") as f:
    f.write("group\tclass\tvietnamese\n")
    for g in sorted(groups):
        for e in groups[g]:
            f.write(f"{g}\t{e['class']}\t{e['text'].replace(chr(9), ' ')}\n")
print({k: len(v) for k, v in sorted(groups.items())}, out["count"])
PY
```


## Pemulihan nama file Kotlin

`SMAP` tersedia di `reverse_engineer/out/raw/DebugProbesKt.bin` dan inline dalam
smali; nama file di smali memakai literal escaped (`\n`).

```bash
python3 - <<'PY'
import re, glob
from collections import Counter
names = Counter()
pat = re.compile(r'\+ \d+ ([A-Za-z0-9_.\-/]+\.kt)\\n')
for p in glob.glob("reverse_engineer/out/apktool/smali*/vn/wahu/vhwuwa/**/*.smali", recursive=True):
    for m in pat.finditer(open(p, encoding="utf-8", errors="replace").read()):
        names[m.group(1)] += 1
for n, c in sorted(names.items()):
    print(f"{c:5d}  {n}")
PY
```

Output mencakup file Kotlin aplikasi dan file stdlib Kotlin/Compose.

## Output

```text
reverse_engineer/
├── strings/       # vi-strings.json, vi-strings.tsv
├── tools/         # wrapper dan helper analisis
├── <APK sumber>   # third-party; disediakan terpisah, gitignored
└── out/           # hasil reproduksi, gitignored
    ├── raw/       # unzip APK, termasuk DebugProbesKt.bin
    ├── apktool/   # manifest, resources, smali*/
    └── jadx/      # opsional
```