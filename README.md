# Craft apps for macOS 10.15 Catalina (unofficial)

Unofficial community project that builds the Craft apps from
[storytold](https://github.com/storytold) for **macOS 10.15 Catalina on Intel Macs**. It is not
affiliated with or endorsed by the original authors. The apps are made by the ArtCraft team and
their contributors and are licensed under MIT OR Apache-2.0.

This repository contains **no app source code and no logos**. It only holds:

- `install.sh`: a script that builds one app of your choice on your own Mac,
- `.github/workflows/catalina-builds.yml`: a GitHub Actions workflow that builds one app on GitHub,
- `patches/`: small source patches for apps that need more than the version change,
- this README.

Builds are **Intel (x86_64) only**, **ad-hoc signed and not notarized**. Apple silicon Macs do not
need this project: the original apps run there on macOS 11 and newer.

## What is changed

1. **Minimum macOS version lowered from 11.0 to 10.15** in `packaging/macos/package.sh`
   (`MACOSX_DEPLOYMENT_TARGET`) and `packaging/macos/Info.plist.in` (`LSMinimumSystemVersion`).
2. **App-specific patches** from `patches/<app>.patch`, applied to every new version of that app.
   If a new upstream version breaks a patch, the build stops with a clear message and the patch has
   to be fixed.

The latest stable release (`v*` tag) of each app is built; pre-releases are only used when an app
has nothing else.

## Status

Initially tested in November 2026 with the then-current releases (for example PhotoCraft 0.23.1).
Newer versions are built the same way, but only the first tested version is known to work.

"Basic tests" means: the app starts, and documents can be opened and saved. Other functions are
**not** tested. Linking an app for 10.15 does not prove that every function exists there; a
function from a newer macOS can still crash the app when it is used.

| App | Patch | Tested on Catalina |
|---|---|---|
| AniCraft | none | not tested |
| CADCraft | none | not tested |
| Craft Anything | none | not tested |
| DeckCraft | none | not tested |
| DesignCraft | none | basic tests |
| EffectCraft | none | not tested |
| FigCraft | none | not tested |
| FilmCraft | none | basic tests |
| GridCraft | none | basic tests |
| LightCraft | none | not tested |
| MaterialCraft | none | not tested |
| PdfCraft | none | basic tests |
| PhotoCraft | `patches/photocraft.patch`: the screen color picker called two system functions that exist only from macOS 11 and crashed on 10.15; the patch skips them there | basic tests, screen color picker |
| SoundCraft | none | not tested |
| TextureCraft | none | not tested |
| VectorCraft | none | not tested |
| WordCraft | none | basic tests |

Please report test results (what works, what crashes) as an issue.

## Build on your Mac with `install.sh`

Everything is typed into the **Terminal** app (Applications > Utilities > Terminal). You need a
Mac with macOS 10.15 and an internet connection. The first build of an app takes a long time (30
minutes or more on an older Mac) and needs several GB of free disk space. The script builds **one
app per run**.

```
git clone https://github.com/sudo-mk-sandwich/craft-catalina.git
cd craft-catalina
bash install.sh
```

If macOS opens a window and offers to install the developer tools when you type `git`, accept,
wait until it is finished, and type the commands again.

The script then:

1. shows a menu: choose the app with the **arrow keys** (or `j`/`k`), start with **Enter**, leave
   with `q`. Apps marked `[basic tests done]` were tested on Catalina,
2. checks whether the Apple developer tools are installed (and starts their installation if not),
3. checks whether Rust is installed. If not, it explains what it will do and asks before it runs
   the official installer from https://sh.rustup.rs. It also updates Rust and adds the Intel target,
4. clones the app and the translations (`craft-i18n`) into `~/craft-build`,
5. checks out the latest stable release, lowers the minimum macOS version to 10.15 and applies the
   patch for that app if there is one,
6. builds, and opens the folder with the finished app (`target/macos-package`). The path of the
   disk image (`.dmg`) is printed as well.

It **stops at the first error** so that it can be fixed; run it again afterwards.

Drag the app into your Applications folder. Because it is not signed by Apple, open it the first
time with **right-click > Open**, then confirm.

Useful variants:

```
bash install.sh wordcraft                 # skip the menu
CRAFT_REF=v0.5.0 bash install.sh wordcraft   # build a specific tag or branch
CRAFT_BUILD_DIR=/Volumes/Big/craft bash install.sh   # build somewhere else
```

**Update:** run `git pull` in the `craft-catalina` folder (new patches), then `bash install.sh`
again. The script fetches the latest release of the app and discards local changes in its own build
folder. Build output is kept, so later builds are faster.

## Build on GitHub (no Mac needed)

1. Fork this repository.
2. Open the **Actions** tab, enable workflows, choose **Catalina builds (manual)** and
   **Run workflow**.
3. Pick the app in the list. Leave "ref" empty to build the latest stable release.
4. When the run is finished, the `.dmg` is attached to the run as an artifact (kept for 3 days).

The GitHub build uses a newer macOS SDK than the one on a Catalina Mac. It shows whether an app
compiles, but not whether it runs on Catalina; test the result on a real Catalina Mac.

## Troubleshooting

- **`Undefined symbols ... _CG...` while linking:** the app calls a system function that Catalina
  does not have. That needs a patch for this app. Please open an issue with the last 30 lines of the
  output. (PhotoCraft gets a linker setting from `patches/photocraft.cargo-config.toml` for this.)
- **`patches/<app>.patch does not apply`:** the app changed the code the patch touches. Please open
  an issue; build an older release meanwhile with `CRAFT_REF=<tag>`.
- **`MACOSX_DEPLOYMENT_TARGET is not 10.15 ... after the change`:** upstream changed the lines the
  script edits (for example to a higher version). Please open an issue.
- **An app has no release yet** (for example a brand new app): name a branch yourself, for example
  `CRAFT_REF=main bash install.sh <app>`.
- **The app crashes when a certain function is used:** start it from the Terminal to see the
  message, for example `/Applications/WordCraft.app/Contents/MacOS/WordCraft`, and open an issue
  with the output.

## Build by hand (alternative to the script)

For people who want to see every step. Replace `wordcraft` with the app you want.

```
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Press Enter for the default installation of Rust, then **close the Terminal window and open a new
one**, and add the Intel target:

```
rustup target add x86_64-apple-darwin
mkdir -p ~/craft-build
cd ~/craft-build
git clone https://github.com/storytold/craft-i18n.git
git clone https://github.com/storytold/wordcraft.git
git clone https://github.com/sudo-mk-sandwich/craft-catalina.git
cd wordcraft
git checkout "$(git tag --list 'v[0-9]*' --sort=-v:refname | grep -v -- '-' | head -n 1)"
```

Lower the minimum version, then apply the patch if `../craft-catalina/patches/wordcraft.patch`
exists (for PhotoCraft also copy `../craft-catalina/patches/photocraft.cargo-config.toml` to
`.cargo/config.toml` after `mkdir -p .cargo`):

```
perl -pi -e 's/MACOSX_DEPLOYMENT_TARGET=11\.0/MACOSX_DEPLOYMENT_TARGET=10.15/' packaging/macos/package.sh
perl -0pi -e 's|(LSMinimumSystemVersion</key>\s*<string>)11\.0|${1}10.15|' packaging/macos/Info.plist.in
git apply ../craft-catalina/patches/wordcraft.patch
CRAFT_I18N_DIR="$PWD/../craft-i18n" ./packaging/macos/package.sh --arch x86_64
open target/macos-package
```

(Leave out the `git apply` line if there is no patch for the app.)

## License

The code that gets built is under MIT OR Apache-2.0; see the original repositories for
`LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE`. Those files are also bundled in the apps. This
repository's own files (script, workflow, patches, README) are provided as is, without warranty.
