# Craft apps for macOS 10.15 Catalina (unofficial)

Unofficial community project that builds the Craft apps from
[storytold](https://github.com/storytold) for **macOS 10.15 Catalina on Intel Macs**. It is not
affiliated with or endorsed by the original authors. The apps are made by the ArtCraft team and
their contributors and are licensed under MIT OR Apache-2.0.

This repository contains **no app source code and no logos**. It only holds:

- `.github/workflows/catalina-builds.yml`: a GitHub Actions workflow that checks out the original
  repository of an app, lowers the minimum macOS version and builds it,
- `patches/<app>.patch`: small source patches for apps that need more than the version change,
- this README.

Builds are **Intel (x86_64) only**, **ad-hoc signed and not notarized**. Apple silicon Macs do not
need this project: the original apps run there on macOS 11 and newer.

## What is changed

1. **Minimum macOS version lowered from 11.0 to 10.15** in `packaging/macos/package.sh`
   (`MACOSX_DEPLOYMENT_TARGET`) and `packaging/macos/Info.plist.in` (`LSMinimumSystemVersion`).
2. **App-specific patches** (see the table).

Features that need newer system APIs than Catalina has can still fail at runtime. Linking a
newer-SDK build for 10.15 does not prove that every function works there; every app needs a test
on a real Catalina Mac.

## Status

| App | Patch | Tested on Catalina |
|---|---|---|
| PhotoCraft 0.23.1 | `patches/photocraft.patch`: the screen color picker used two system functions that exist only from macOS 11 and crashed on 10.15; the patch skips them there | start, open and save, screen color picker |
| all others | none yet | not tested |

Please report test results (what works, what crashes) as an issue.

## Build with GitHub (no Mac needed)

1. Fork this repository (or use it directly if you are the owner).
2. Open the **Actions** tab, enable workflows, choose **Catalina builds (manual)** and
   **Run workflow**.
3. Enter the app repo name (for example `photocraft` or `wordcraft`), several separated by commas,
   or `all`. Leave "ref" empty to build the latest `v*` tag of each app.
4. When the run is finished, the `.dmg` is attached to the run as an artifact.

## Build it yourself (step by step)

Everything below is typed into the **Terminal** app (Applications > Utilities > Terminal).
The first build of an app takes a long time (30 minutes or more on an older Mac) and needs several
GB of free disk space. Replace `photocraft` with the name of the app you want.

### 1. Apple developer tools

```
xcode-select --install
```

If it says the software is not available for Catalina, download **Command Line Tools for Xcode
12.4** from https://developer.apple.com/download/all/ (a free Apple ID is enough), install the
`.dmg` and check with `xcrun --show-sdk-version`, which should print `10.15` or higher.

### 2. Rust

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Press Enter for the default installation, then **close the Terminal window and open a new one**.
Check with `cargo --version`.

### 3. Get the code

```
mkdir -p ~/craft-build
cd ~/craft-build
git clone https://github.com/storytold/craft-i18n.git
git clone https://github.com/storytold/photocraft.git
git clone https://github.com/sudo-mk-sandwich/craft-catalina.git
cd photocraft
git checkout v0.23.1
```

`craft-i18n` holds the translations; without it the app is English only. Use the version you want
to build instead of `v0.23.1`; the patch was tested with that version, others may no longer match.

### 4. Apply the changes

```
sed -i '' 's/MACOSX_DEPLOYMENT_TARGET=11\.0/MACOSX_DEPLOYMENT_TARGET=10.15/' packaging/macos/package.sh
perl -0pi -e 's|(LSMinimumSystemVersion</key>\s*<string>)11\.0|${1}10.15|' packaging/macos/Info.plist.in
```

If `../craft-catalina/patches/photocraft.patch` exists, apply it:

```
git apply ../craft-catalina/patches/photocraft.patch
```

Check with `git diff --stat`: it should list `package.sh`, `Info.plist.in` and, for PhotoCraft,
`screen_color.rs`.

### 5. PhotoCraft only: linker setting for the 10.15 SDK

If your developer tools contain the macOS 10.15 SDK, the linker does not know the two functions
mentioned in the status table and stops with `Undefined symbols ... _CGPreflightScreenCaptureAccess`.
This setting makes it resolve them at runtime (the patch makes sure they are only called where
they exist). Run the two lines one after the other:

```
mkdir -p .cargo
```

```
printf '[target.x86_64-apple-darwin]\nrustflags = [\n  "-C", "link-arg=-Wl,-U,_CGPreflightScreenCaptureAccess",\n  "-C", "link-arg=-Wl,-U,_CGRequestScreenCaptureAccess",\n]\n' > .cargo/config.toml
```

Do not use this for other apps. If another app stops with `Undefined symbols`, it uses a system
function that Catalina does not have, which needs a patch; please open an issue with the message.

### 6. Build

```
CRAFT_I18N_DIR="$PWD/../craft-i18n" ./packaging/macos/package.sh --arch x86_64
```

Warnings are normal. Stop only at lines that start with `error`.

### 7. Where is the app?

- The app: `target/macos-package/` inside the app's folder, for example
  `~/craft-build/photocraft/target/macos-package/`. Open it with `open target/macos-package`.
- A disk image (`.dmg`) is also created; the last lines of the build output print its path.

Drag the app into your Applications folder. Because it is not signed by Apple, open it the first
time with **right-click > Open**, then confirm.

## License

The code that gets built is under MIT OR Apache-2.0, see the original repositories for
`LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE`; those files are also bundled in the apps. This
repository's own files (workflow, patches, README) are provided as is, without warranty.
