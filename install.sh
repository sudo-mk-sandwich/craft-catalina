#!/usr/bin/env bash
# Build ONE Craft app from source for macOS 10.15 Catalina (Intel).
#
#   bash install.sh              pick the app in a menu (arrow keys, Enter)
#   bash install.sh wordcraft    build that app directly
#
# What it does: checks Apple developer tools and Rust (installs Rust after asking), clones the app
# and craft-i18n (translations) into ~/craft-build, checks out the latest stable release tag,
# lowers the minimum macOS version to 10.15, applies patches/<app>.patch if there is one, builds,
# and opens the folder with the finished app. It stops at the first error so it can be fixed.
#
# Environment (all optional):
#   CRAFT_BUILD_DIR   where the sources and builds live (default ~/craft-build)
#   CRAFT_REF         build this tag or branch instead of the latest stable tag
#   CRAFT_GIT_BASE    where the repositories are cloned from (default https://github.com/storytold)
#
# Written for the bash 3.2 that ships with macOS 10.15: no associative arrays, no mapfile.
set -eu
set -o pipefail

GIT_BASE="${CRAFT_GIT_BASE:-https://github.com/storytold}"
WORK="${CRAFT_BUILD_DIR:-$HOME/craft-build}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

APPS=(anicraft cadcraft craft-anything deckcraft designcraft effectcraft figcraft filmcraft gridcraft lightcraft materialcraft pdfcraft photocraft soundcraft texturecraft vectorcraft wordcraft)
DESCR=(
  "2D vector animation"
  "CAD and drafting"
  "Software exploration and creation (early alpha)"
  "Presentations"
  "Page layout and publishing"
  "Motion graphics and VFX"
  "Interface design and prototyping"
  "Video editing"
  "Spreadsheets and data analysis"
  "Photo library and RAW developer"
  "Procedural materials"
  "PDF tool"
  "Image editing"
  "Digital audio workstation"
  "3D texture painting"
  "Vector illustration"
  "Word processor"
)
# Apps with basic tests on Catalina (start, open and save). Shown in the menu.
TESTED=" designcraft filmcraft gridcraft pdfcraft photocraft wordcraft "

say() { printf '\n==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() { printf '\nerror: %s\n' "$*" >&2; exit 1; }
cursor_on() { if [ -t 1 ]; then printf '\033[?25h'; fi; }
trap cursor_on EXIT
trap 'cursor_on; printf "\n"; exit 130' INT

ask() { # ask "question" -> returns 0 for yes (default), 1 for no
  local ans=""
  printf '%s [Y/n] ' "$1"
  read -r ans || ans=""
  case "$ans" in n | N | no | No | NO) return 1 ;; *) return 0 ;; esac
}

# ---- menu --------------------------------------------------------------------------------------
CHOSEN=""

app_label() { # app_label index
  local i="$1" mark=""
  case "$TESTED" in *" ${APPS[$i]} "*) mark="  [basic tests done]" ;; esac
  printf '%-15s %s%s' "${APPS[$i]}" "${DESCR[$i]}" "$mark"
}

choose_app() {
  local n="${#APPS[@]}" sel=0 i key rest
  if [ ! -t 0 ] || [ ! -t 1 ]; then
    i=0
    while [ "$i" -lt "$n" ]; do
      printf '%2d) %s\n' "$((i + 1))" "$(app_label "$i")"
      i=$((i + 1))
    done
    printf 'Number of the app to build: '
    read -r key || die "no app chosen"
    case "$key" in '' | *[!0-9]*) die "not a number: $key" ;; esac
    [ "$key" -ge 1 ] && [ "$key" -le "$n" ] || die "number out of range: $key"
    CHOSEN="${APPS[$((key - 1))]}"
    return
  fi

  printf 'Which app do you want to build?  (arrow keys or j/k, Enter = build, q = quit)\n\n'
  printf '\033[?25l'
  draw_menu() {
    local j=0
    while [ "$j" -lt "$n" ]; do
      if [ "$j" -eq "$sel" ]; then
        printf '\033[2K \033[7m> %s\033[0m\n' "$(app_label "$j")"
      else
        printf '\033[2K   %s\n' "$(app_label "$j")"
      fi
      j=$((j + 1))
    done
  }
  draw_menu
  while true; do
    key=""
    IFS= read -rsn1 key || break
    case "$key" in
      $'\033')
        rest=""
        read -rsn2 -t 1 rest || true
        case "$rest" in
          '[A') sel=$(((sel + n - 1) % n)) ;;
          '[B') sel=$(((sel + 1) % n)) ;;
        esac
        ;;
      k) sel=$(((sel + n - 1) % n)) ;;
      j) sel=$(((sel + 1) % n)) ;;
      q | Q) cursor_on; printf '\n'; exit 0 ;;
      '') break ;;
    esac
    printf '\033[%dA' "$n"
    draw_menu
  done
  cursor_on
  CHOSEN="${APPS[$sel]}"
  printf '\n'
}

# ---- prerequisites -----------------------------------------------------------------------------
ensure_dev_tools() {
  if xcode-select -p >/dev/null 2>&1 && xcrun --show-sdk-version >/dev/null 2>&1; then
    say "Apple developer tools: found (SDK $(xcrun --show-sdk-version))"
    return
  fi
  say "Apple developer tools are missing"
  echo "A window will open; click Install and wait until it is finished."
  xcode-select --install || true
  die "Run this script again after the installation. If macOS says the software is not available for Catalina, download 'Command Line Tools for Xcode 12.4' from https://developer.apple.com/download/all/ (free Apple ID), install the .dmg and run this script again."
}

ensure_rust() {
  if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
  if command -v cargo >/dev/null 2>&1 && command -v rustup >/dev/null 2>&1; then
    say "Rust: found ($(cargo --version))"
  else
    say "Rust is not installed"
    echo "Rust is needed to build the apps. It will be installed with the official installer from"
    echo "https://sh.rustup.rs (into ~/.cargo and ~/.rustup, default settings, no sudo)."
    ask "Install Rust now?" || die "Rust is required. Install it from https://rustup.rs and run this script again."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || die "The Rust installation failed."
    if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
    command -v cargo >/dev/null 2>&1 || die "cargo was not found after the installation. Open a new Terminal window and run this script again."
  fi
  say "Updating Rust and adding the Intel target"
  rustup update stable || die "rustup update failed"
  rustup target add x86_64-apple-darwin || die "adding the Intel target failed"
}

# ---- sources -----------------------------------------------------------------------------------
sync_repo() { # sync_repo <name>  (clone or update, no checkout of a tag)
  local name="$1"
  if [ ! -d "$WORK/$name/.git" ]; then
    say "Cloning $name"
    git clone "$GIT_BASE/$name.git" "$WORK/$name" || die "cloning $GIT_BASE/$name.git failed. Is the repository name right, and is the network up?"
  fi
}

pick_ref() { # prints the ref to build: CRAFT_REF, else latest stable v* tag, else latest v* tag
  local ref="${CRAFT_REF:-}" tags
  if [ -z "$ref" ]; then
    tags="$(git tag --list 'v[0-9]*' --sort=-v:refname)"
    ref="$(printf '%s\n' "$tags" | grep -v -- '-' | head -n 1 || true)"
    if [ -z "$ref" ]; then ref="$(printf '%s\n' "$tags" | head -n 1 || true)"; fi
  fi
  printf '%s' "$ref"
}

die_build() {
  die "The build stopped (see the lines above, usually the first one starting with 'error').
  The sources stay in $APPDIR; run this script again to continue after a fix.
  'Undefined symbols' means the app calls a system function Catalina does not have. That needs a
  patch for this app: please open an issue with the last 30 lines of the output."
}

# ---- main --------------------------------------------------------------------------------------
[ "$(uname -s)" = "Darwin" ] || die "This script is for macOS."
if [ "$(uname -m)" != "x86_64" ]; then
  warn "This Mac is not an Intel Mac. The apps already run on Apple silicon with macOS 11 or newer."
  ask "Build an Intel version anyway?" || exit 0
fi

if [ "$#" -ge 1 ]; then
  APP="$(printf '%s' "$1" | tr 'A-Z' 'a-z')"
  found=0
  for a in "${APPS[@]}"; do if [ "$a" = "$APP" ]; then found=1; fi; done
  [ "$found" -eq 1 ] || die "unknown app: $1 (valid: ${APPS[*]})"
else
  choose_app
  APP="$CHOSEN"
fi
say "App: $APP"

ensure_dev_tools
ensure_rust

mkdir -p "$WORK"
avail_kb="$(df -k "$WORK" | awk 'NR==2 {print $4}')"
case "$avail_kb" in '' | *[!0-9]*) ;; *) if [ "$avail_kb" -lt 10485760 ]; then warn "less than 10 GB free in $WORK; a build needs several GB"; fi ;; esac

sync_repo craft-i18n
git -C "$WORK/craft-i18n" pull --ff-only || warn "could not update craft-i18n; using the version already there"

sync_repo "$APP"
APPDIR="$WORK/$APP"
cd "$APPDIR"
git fetch --tags --force origin || die "git fetch failed"
# The build folder belongs to this script: discard changes from an earlier run (build output in
# target/ is kept so later builds stay fast).
git reset --hard -q
git clean -fdq -e /target

REF="$(pick_ref)"
[ -n "$REF" ] || die "$APP has no release tag (v*) yet. Name a branch or tag yourself, for example: CRAFT_REF=main bash install.sh $APP"
if git rev-parse -q --verify "refs/tags/$REF" >/dev/null; then
  git checkout -q --detach "refs/tags/$REF" || die "checkout of $REF failed"
elif git rev-parse -q --verify "refs/remotes/origin/$REF" >/dev/null; then
  git checkout -q --detach "refs/remotes/origin/$REF" || die "checkout of $REF failed"
else
  die "ref not found: $REF"
fi
say "Building $APP at $REF ($(git log -1 --format=%h))"

[ -f packaging/macos/package.sh ] || die "packaging/macos/package.sh not found in $APP; this app is built differently and needs its own handling."
[ -f packaging/macos/Info.plist.in ] || die "packaging/macos/Info.plist.in not found in $APP; this app is built differently and needs its own handling."

say "Lowering the minimum macOS version to 10.15"
perl -pi -e 's/MACOSX_DEPLOYMENT_TARGET=11\.0/MACOSX_DEPLOYMENT_TARGET=10.15/' packaging/macos/package.sh
perl -0pi -e 's|(LSMinimumSystemVersion</key>\s*<string>)11\.0|${1}10.15|' packaging/macos/Info.plist.in
grep -q 'MACOSX_DEPLOYMENT_TARGET=10.15' packaging/macos/package.sh || die "MACOSX_DEPLOYMENT_TARGET is not 10.15 in package.sh after the change. Upstream may have changed that line; look at it with: grep -n MACOSX_DEPLOYMENT_TARGET $APPDIR/packaging/macos/package.sh"
grep -A1 'LSMinimumSystemVersion' packaging/macos/Info.plist.in | grep -q '<string>10.15</string>' || die "LSMinimumSystemVersion is not 10.15 in Info.plist.in after the change. Upstream may have changed that line."

PATCH="$SCRIPT_DIR/patches/$APP.patch"
if [ -f "$PATCH" ]; then
  say "Applying patches/$APP.patch"
  git apply "$PATCH" || die "patches/$APP.patch does not apply to $REF. The app changed the code the patch touches; the patch needs an update. Please open an issue."
else
  echo "No patch for $APP; building with the version change only."
fi

CFG="$SCRIPT_DIR/patches/$APP.cargo-config.toml"
if [ -f "$CFG" ]; then
  say "Adding linker settings from patches/$APP.cargo-config.toml"
  if [ -n "$(git ls-files .cargo)" ]; then
    die "$APP already has a .cargo folder; merge $CFG into .cargo/config.toml by hand."
  fi
  mkdir -p .cargo
  cp "$CFG" .cargo/config.toml
fi

say "Building (the first build of an app takes a long time; warnings are normal)"
CRAFT_I18N_DIR="$WORK/craft-i18n" ./packaging/macos/package.sh --arch x86_64 || die_build

OUT="$(find "$APPDIR/target" -maxdepth 3 -type d -name macos-package 2>/dev/null | head -n 1 || true)"
if [ -z "$OUT" ]; then OUT="$APPDIR"; fi
DMG="$(find "$APPDIR" -maxdepth 4 -name '*-macos-x86_64.dmg' 2>/dev/null | head -n 1 || true)"
say "Done"
echo "App folder: $OUT"
if [ -n "$DMG" ]; then echo "Disk image: $DMG"; fi
echo "Because the app is not signed by Apple, open it the first time with right-click > Open."
open "$OUT" || true
