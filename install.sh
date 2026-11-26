#!/usr/bin/env bash
# Build ONE Craft app from source for macOS 10.15 Catalina (Intel).
#
#   bash install.sh              pick the app in a menu (arrow keys, Enter)
#   bash install.sh wordcraft    build that app directly
#
# For programs that drive this script (such as a graphical front end) there are also actions that
# print machine-readable lines starting with "@@" and never ask questions:
#   --list-apps            @@APP <name> <tested 0|1> <description>   (tab separated)
#   --installed            @@INSTALLED_APP <name> <version> <path> for each installed app
#   --check                @@CHECK tools|rust|target|disk|arch|macos <status> [detail]
#   --install-tools        opens Apple's installer for the developer tools
#   --install-rust         installs Rust (official installer, no sudo) and the Intel target
#   --machine --build APP  build one app; progress as @@STEP, result as @@DONE <app> or @@ERROR <text>
#   --info APP             @@APP_PATH, @@BUILT_VERSION, @@SIZE_KB, @@INSTALLED yes|no [path]
#   --copy APP             copy the built app to /Applications (replaces an older copy)
#   --clean APP            delete the sources and build output of APP (@@CLEANED <KB>)
#
# What it does: first updates itself and the patches (git pull in the folder it lives in), then
# checks Apple developer tools and Rust (installs Rust after asking), clones the app
# and craft-i18n (translations) into ~/craft-build, checks out the latest stable release tag,
# lowers the minimum macOS version to 10.15, applies patches/<app>.patch if there is one, builds,
# and opens the folder with the finished app. It stops at the first error so it can be fixed.
#
# Environment (all optional):
#   CRAFT_BUILD_DIR   where the sources and builds live (default ~/craft-build)
#   CRAFT_REF         build this tag or branch instead of the latest stable tag
#   CRAFT_GIT_BASE    where the repositories are cloned from (default https://github.com/storytold)
#   CRAFT_NO_UPDATE   set to 1 to skip the git pull of this script's own repository (offline use)
#   CRAFT_APPLICATIONS_DIR  where --copy puts the app (default /Applications)
#   CRAFT_DMG         set to 1 to also build the disk image (.dmg); by default the build stops once the app is
#                     finished (assembled and signed), which is faster
#   CRAFT_SKIP_CLI    set to 1 to leave out the command-line tool of an app, where the app's script allows it
#                     (faster; works together with CRAFT_DMG=1)
#   CRAFT_INSTALL_DIR       an extra folder: installed apps are looked for here first, and an app that is
#                           not installed yet is copied here instead of /Applications
#
# Written for the bash 3.2 that ships with macOS 10.15: no associative arrays, no mapfile.
set -eu
set -o pipefail

GIT_BASE="${CRAFT_GIT_BASE:-https://github.com/storytold}"
WORK="${CRAFT_BUILD_DIR:-$HOME/craft-build}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SELF="$SCRIPT_DIR/$(basename "${BASH_SOURCE[0]}")"

APPS=(anicraft cadcraft craft-anything deckcraft designcraft effectcraft figcraft filmcraft gridcraft lightcraft mailcraft materialcraft pdfcraft photocraft solidcraft soundcraft texturecraft vectorcraft wordcraft)
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
  "Mail, calendar, contacts and tasks (other authors)"
  "Procedural materials"
  "PDF tool"
  "Image editing"
  "3D CAD (other authors)"
  "Digital audio workstation"
  "3D texture painting"
  "Vector illustration"
  "Word processor"
)
# Apps with basic tests on Catalina (start, open and save). Shown in the menu.
TESTED=" anicraft cadcraft deckcraft designcraft effectcraft figcraft filmcraft gridcraft lightcraft materialcraft pdfcraft photocraft solidcraft soundcraft texturecraft vectorcraft wordcraft "

MACHINE=0
emit() { printf '@@%s\n' "$*"; }
say() { printf '\n==> %s\n' "$*"; if [ "$MACHINE" = 1 ]; then emit "STEP $*"; fi; }
warn() { printf 'warning: %s\n' "$*" >&2; if [ "$MACHINE" = 1 ]; then emit "WARN $*"; fi; }
die() {
  printf '\nerror: %s\n' "$*" >&2
  if [ "$MACHINE" = 1 ]; then emit "ERROR $(printf '%s' "$*" | head -n 1)"; fi
  exit 1
}
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
  if [ "$MACHINE" = 1 ]; then die "The Apple developer tools are missing."; fi
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
    if [ "$MACHINE" = 1 ]; then die "Rust is missing."; fi
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
repo_url() { # repo_url <name>: two apps of the suite live outside the storytold account
  case "$1" in
    mailcraft) printf '%s' "${CRAFT_MAILCRAFT_URL:-https://github.com/UCS-Care-Group/mailcraft.git}" ;;
    solidcraft) printf '%s' "${CRAFT_SOLIDCRAFT_URL:-https://github.com/tangency-labs/solidcraft.git}" ;;
    *) printf '%s/%s.git' "$GIT_BASE" "$1" ;;
  esac
}

sync_repo() { # sync_repo <name>  (clone or update, no checkout of a tag)
  local name="$1" url
  url="$(repo_url "$name")"
  if [ ! -d "$WORK/$name/.git" ]; then
    say "Cloning $name"
    git clone "$url" "$WORK/$name" || die "cloning $url failed. Is the repository name right, and is the network up?"
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
  if [ "$MACHINE" = 1 ]; then
    emit "LOG $LOG"
    if grep -q 'Undefined symbols' "$LOG" 2>/dev/null; then emit "KIND undefined-symbols"
    elif grep -q 'framework not found' "$LOG" 2>/dev/null; then emit "KIND framework-not-found"
    elif grep -q 'does not apply' "$LOG" 2>/dev/null; then emit "KIND patch"
    else emit "KIND build-failed"; fi
    die "The build stopped."
  fi
  die "The build stopped (see the lines above, usually the first one starting with 'error').
  The sources stay in $APPDIR; run this script again to continue after a fix.
  'Undefined symbols' means the app calls a system function Catalina does not have. That needs a
  patch for this app: please open an issue with the last 30 lines of the output."
}

# ---- self update -------------------------------------------------------------------------------
# Pull the repository this script lives in, so new patches and script fixes are used without any
# extra step. If the pull changed this very file, start the new version (once) instead of running
# on with the old one.
self_update() {
  local before after out
  if [ "${CRAFT_NO_UPDATE:-}" = "1" ] || [ "${CRAFT_SELF_UPDATED:-}" = "1" ]; then return 0; fi
  if ! git -C "$SCRIPT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "(This folder is not a git clone, so the script and patches are not updated automatically.)"
    return 0
  fi
  say "Checking for updates of this script and the patches (git pull)"
  before="$(cksum < "$SELF")"
  if out="$(LC_ALL=C git -C "$SCRIPT_DIR" pull --ff-only 2>&1)"; then
    printf '%s\n' "$out"
    after="$(cksum < "$SELF")"
    if [ "$before" != "$after" ]; then
      say "install.sh was updated; restarting it"
      CRAFT_SELF_UPDATED=1 exec bash "$SELF" "$@"
    fi
  else
    printf '%s\n' "$out" >&2
    warn "git pull failed; continuing with the local version of the script and patches."
    case "$out" in
      *untracked*) warn "You created a file in patches/ by hand that now exists on GitHub: delete your copy (rm patches/<file>) and run this script again." ;;
      *"local changes"*) warn "You changed a file in this folder by hand. Undo it with: git checkout -- <file> (or git stash), then run this script again." ;;
    esac
  fi
}

# ---- actions for programs that drive this script ---------------------------------------------
valid_app() { # valid_app <name>
  local a
  for a in "${APPS[@]}"; do if [ "$a" = "$1" ]; then return 0; fi; done
  return 1
}

bundle_of() { # bundle_of <app>: the finished .app of that app, if any
  ls -d "$WORK/$1/target/macos-package/"*.app 2>/dev/null | head -n 1 || true
}

plist_version() { # plist_version <bundle>
  /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$1/Contents/Info.plist" 2>/dev/null || true
}

action_list_apps() {
  local i=0 t
  while [ "$i" -lt "${#APPS[@]}" ]; do
    t=0
    case "$TESTED" in *" ${APPS[$i]} "*) t=1 ;; esac
    printf '@@APP\t%s\t%s\t%s\n' "${APPS[$i]}" "$t" "${DESCR[$i]}" || return 0
    i=$((i + 1))
  done
}

action_check() {
  local dir kb gb
  if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
  if xcode-select -p >/dev/null 2>&1 && xcrun --show-sdk-version >/dev/null 2>&1; then
    emit "CHECK tools ok $(xcrun --show-sdk-version)"
  else
    emit "CHECK tools missing"
  fi
  if command -v cargo >/dev/null 2>&1 && command -v rustup >/dev/null 2>&1; then
    emit "CHECK rust ok $(cargo --version | awk '{print $2}')"
    if rustup target list --installed 2>/dev/null | grep -q '^x86_64-apple-darwin$'; then
      emit "CHECK target ok"
    else
      emit "CHECK target missing"
    fi
  else
    emit "CHECK rust missing"
    emit "CHECK target missing"
  fi
  dir="$WORK"
  while [ ! -d "$dir" ] && [ "$dir" != "/" ]; do dir="$(dirname "$dir")"; done
  kb="$(df -k "$dir" | awk 'NR==2 {print $4}')"
  case "$kb" in
    '' | *[!0-9]*) emit "CHECK disk unknown" ;;
    *)
      gb=$((kb / 1048576))
      if [ "$kb" -lt 10485760 ]; then emit "CHECK disk low $gb"; else emit "CHECK disk ok $gb"; fi
      ;;
  esac
  if [ "$(uname -m)" = "x86_64" ]; then emit "CHECK arch intel"; else emit "CHECK arch other"; fi
  emit "CHECK macos $(sw_vers -productVersion 2>/dev/null || echo unknown)"
}

action_install_tools() {
  if xcode-select -p >/dev/null 2>&1 && xcrun --show-sdk-version >/dev/null 2>&1; then
    emit "INFO The developer tools are already installed."
    return 0
  fi
  xcode-select --install || true
  emit "INFO Apple's installer window was opened. Click Install and wait until it is finished."
}

action_install_rust() {
  if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
  if ! { command -v cargo >/dev/null 2>&1 && command -v rustup >/dev/null 2>&1; }; then
    say "Installing Rust (official installer from https://sh.rustup.rs, no sudo)"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || die "The Rust installation failed."
    if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
    command -v cargo >/dev/null 2>&1 || die "cargo was not found after the installation."
  fi
  say "Adding the Intel target"
  rustup target add x86_64-apple-darwin || die "adding the Intel target failed"
  emit "DONE rust"
}

action_info() { # action_info <app>
  local b kb dest
  b="$(bundle_of "$1")"
  kb="$(du -sk "$WORK/$1" 2>/dev/null | awk '{print $1}')"
  emit "SIZE_KB ${kb:-0}"
  if [ -n "$b" ]; then
    emit "APP_PATH $b"
    emit "BUILT_VERSION $(plist_version "$b")"
    dest="$(find_installed "$1")"
    if [ -n "$dest" ] && [ -d "$dest" ]; then
      emit "INSTALLED yes $dest"
      emit "INSTALLED_VERSION $(plist_version "$dest")"
    else
      emit "INSTALLED no"
    fi
  fi
}

# Which app of the suite is this bundle? The answer does not depend on the file name, so a renamed
# app (Acrobat.app = PdfCraft) is still recognized: the last part of its bundle identifier
# (ai.storyteller.pdfcraft) names the app. Only a bundle without a readable identifier is matched
# by its file name, written in lower case without spaces or hyphens (WordCraft.app = wordcraft,
# "Craft Anything.app" = craft-anything).
norm_name() { printf '%s' "$1" | tr 'A-Z' 'a-z' | tr -d ' _-'; }

app_of_bundle() { # app_of_bundle <bundle>   prints the app name, or nothing
  local d="$1" id key a
  id="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$d/Contents/Info.plist" 2>/dev/null || true)"
  if [ -n "$id" ]; then key="$(norm_name "${id##*.}")"; else key="$(norm_name "$(basename "$d" .app)")"; fi
  for a in "${APPS[@]}"; do
    if [ "$key" = "$(norm_name "$a")" ]; then printf '%s' "$a"; return 0; fi
  done
  return 0
}

# The places where installed apps are looked for.
# The chosen folder (CRAFT_INSTALL_DIR, if it exists) comes first, so an app found there wins.
app_dirs() {
  local d custom="${CRAFT_INSTALL_DIR:-}" std="${CRAFT_APPLICATIONS_DIR:-/Applications}"
  custom="${custom%/}"
  if [ -n "$custom" ] && [ -d "$custom" ]; then printf '%s\n' "$custom"; fi
  for d in "$std" "$HOME/Applications"; do
    if [ "$d" != "$custom" ]; then printf '%s\n' "$d"; fi
  done
}

# Path of the installed bundle of an app (the first one found), or nothing.
find_installed() { # find_installed <app>
  local dir d
  while IFS= read -r dir; do
    [ -d "$dir" ] || continue
    for d in "$dir"/*.app; do
      [ -d "$d" ] || continue
      if [ "$(app_of_bundle "$d")" = "$1" ]; then printf '%s' "$d"; return 0; fi
    done
  done <<EOF_DIRS
$(app_dirs)
EOF_DIRS
  return 0
}

# Lists the installed apps of the suite: one "@@INSTALLED_APP <name> <version> <path>" line each.
action_installed() {
  local dir d a ver
  while IFS= read -r dir; do
    [ -d "$dir" ] || continue
    for d in "$dir"/*.app; do
      [ -d "$d" ] || continue
      a="$(app_of_bundle "$d")"
      if [ -n "$a" ]; then
        ver="$(plist_version "$d")"
        emit "INSTALLED_APP $a ${ver:-?} $d"
      fi
    done
  done <<EOF_DIRS
$(app_dirs)
EOF_DIRS
  return 0
}

action_copy() { # action_copy <app>
  local b name destdir dest tmp
  b="$(bundle_of "$1")"
  [ -n "$b" ] || die "There is no finished app for $1. Build it first."
  name="$(basename "$b")"
  case "$name" in *.app) ;; *) die "unexpected app name: $name" ;; esac
  dest="$(find_installed "$1")"
  if [ -n "$dest" ]; then
    destdir="$(dirname "$dest")"   # an app the user renamed is updated under its own name
    name="$(basename "$dest")"
  else
    destdir="${CRAFT_INSTALL_DIR:-}"
    destdir="${destdir%/}"
    if [ -z "$destdir" ] || [ ! -d "$destdir" ]; then
      if [ -n "$destdir" ]; then warn "The folder $destdir does not exist; using the Applications folder."; fi
      destdir="${CRAFT_APPLICATIONS_DIR:-/Applications}"
    fi
    dest="$destdir/$name"
  fi
  if pgrep -f "$dest/Contents/MacOS/" >/dev/null 2>&1; then die "$name is running. Quit it and try again."; fi
  tmp="$dest.new.$$"
  rm -rf "$tmp"
  ditto "$b" "$tmp" || { rm -rf "$tmp"; die "Copying to $destdir failed. Do you have permission to write there?"; }
  if [ -d "$dest" ]; then rm -rf "$dest" || die "Could not remove the old $name."; fi
  mv "$tmp" "$dest" || die "Could not move the new $name into place."
  emit "COPIED $dest"
}

action_clean() { # action_clean <app>
  local kb
  [ -n "$WORK" ] && [ "$WORK" != "/" ] && [ "$WORK" != "$HOME" ] || die "refusing to clean in $WORK"
  kb="$(du -sk "$WORK/$1" 2>/dev/null | awk '{print $1}')"
  rm -rf "${WORK:?}/$1" "${WORK:?}/stubs/$1" "${WORK:?}/last-build-$1.log"
  emit "CLEANED ${kb:-0}"
}

# ---- build ---------------------------------------------------------------------------------------
# Makes the build shorter, with two independent options. By default the build script stops once the
# .app is assembled and signed: no disk image (some apps need a newer Python for it). CRAFT_DMG=1
# builds the disk image. CRAFT_SKIP_CLI=1 leaves out the command-line tool, a second program the same
# script compiles, but only where it can be done safely. Every change is made only when the script
# looks as expected; otherwise it stays as it is.
trim_package_script() { # trim_package_script <package.sh>
  local f="$1" tmp="$1.craft-trim" marker what
  if [ "${CRAFT_DMG:-0}" != 1 ]; then
    marker='^# ---- DMG ---'; what="the disk image (CRAFT_DMG=1 builds it)"
  elif [ "${CRAFT_SKIP_CLI:-0}" = 1 ]; then
    marker='^# ---- CLI ---'; what="the command-line tool step"
  else
    return 0
  fi
  if ! grep -q "$marker" "$f"; then
    if [ "${CRAFT_DMG:-0}" != 1 ]; then warn "package.sh has no separate disk image step; building everything it builds."
    else warn "package.sh has no separate command-line tool step; it is built too."; fi
    return 0
  fi
  awk -v m="$marker" -v w="$what" '$0 ~ m && !done { print "echo \"==> skipping " w "\""; print "echo \"==> done\""; print "exit 0"; done=1 } { print }' "$f" > "$tmp" && cat "$tmp" > "$f" && rm -f "$tmp"
  if [ "${CRAFT_DMG:-0}" != 1 ]; then say "Skipping the disk image (CRAFT_DMG=1 builds it)"; fi
  if [ "${CRAFT_SKIP_CLI:-0}" != 1 ]; then return 0; fi
  # The command-line tool: leave it out of the cargo build and of the list of programs to join.
  if grep -Eq ' -p [A-Za-z0-9_-]+-cli' "$f" && grep -Eq '^ *for bin in .* [A-Za-z0-9_-]+-cli; do' "$f"; then
    sed -E -e 's/ -p [A-Za-z0-9_-]+-cli//' -e 's/^( *for bin in .*) [A-Za-z0-9_-]+-cli; do/\1; do/' "$f" > "$tmp"
    # Accept it only when nothing before the new end of the script uses the tool any more.
    if ! grep -Eq ' -p [A-Za-z0-9_-]+-cli' "$tmp" && ! grep -Eq ' [A-Za-z0-9_-]+-cli; do' "$tmp" \
       && ! awk '/^exit 0$/ { exit } { print }' "$tmp" | grep -Eq '/[A-Za-z0-9_-]+-cli["[:space:]]'; then
      cat "$tmp" > "$f"
      say "Leaving out the command-line tool"
    else
      warn "The command-line tool cannot be left out of this app's build; it is built too."
    fi
    rm -f "$tmp"
  else
    warn "The command-line tool cannot be left out of this app's build; it is built too."
  fi
}

do_build() {
  if [ "$(uname -m)" != "x86_64" ]; then
    warn "This Mac is not an Intel Mac. The apps already run on Apple silicon with macOS 11 or newer."
    if [ "$MACHINE" != 1 ]; then ask "Build an Intel version anyway?" || exit 0; fi
  fi

  self_update "$@"

  if [ -n "$ARG_APP" ]; then
    APP="$(printf '%s' "$ARG_APP" | tr 'A-Z' 'a-z')"
    valid_app "$APP" || die "unknown app: $ARG_APP (valid: ${APPS[*]})"
  elif [ "$MACHINE" = 1 ]; then
    die "No app given."
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
  # Where the Info.plist template lives depends on the app.
  PLIST=""
  for p in packaging/macos/Info.plist.in assets/platforms/macos/Info.plist.in; do
    if [ -z "$PLIST" ] && [ -f "$p" ]; then PLIST="$p"; fi
  done
  [ -n "$PLIST" ] || die "No Info.plist.in found in $APP; this app is built differently and needs its own handling."

  say "Lowering the minimum macOS version to 10.15"
  perl -pi -e 's/MACOSX_DEPLOYMENT_TARGET=11\.0/MACOSX_DEPLOYMENT_TARGET=10.15/' packaging/macos/package.sh
  perl -0pi -e 's|(LSMinimumSystemVersion</key>\s*<string>)11\.0|${1}10.15|' "$PLIST"
  grep -q 'MACOSX_DEPLOYMENT_TARGET=10.15' packaging/macos/package.sh || die "MACOSX_DEPLOYMENT_TARGET is not 10.15 in package.sh after the change. Upstream may have changed that line; look at it with: grep -n MACOSX_DEPLOYMENT_TARGET $APPDIR/packaging/macos/package.sh"
  grep -A1 'LSMinimumSystemVersion' "$PLIST" | grep -q '<string>10.15</string>' || die "LSMinimumSystemVersion is not 10.15 in Info.plist.in after the change. Upstream may have changed that line."

  PATCH="$SCRIPT_DIR/patches/$APP.patch"
  if [ -f "$PATCH" ]; then
    say "Applying patches/$APP.patch"
    git apply "$PATCH" || die "patches/$APP.patch does not apply to $REF. The app changed the code the patch touches; the patch needs an update. Please open an issue."
  else
    echo "No source patch for $APP; building with the version change only."
  fi

  CFG="$SCRIPT_DIR/patches/$APP.cargo-config.toml"
  if [ -f "$CFG" ]; then
    say "Adding linker settings from patches/$APP.cargo-config.toml"
    STUBS="$WORK/stubs/$APP"
    mkdir -p "$STUBS"
    # Optional helpers some apps need (see the comments in the cargo-config file):
    # a small C file with replacements for system functions Catalina does not have ...
    SHIMC="$SCRIPT_DIR/patches/$APP.shim.c"
    if [ -f "$SHIMC" ]; then
      clang -c -arch x86_64 -mmacosx-version-min=10.15 -o "$STUBS/shim.o" "$SHIMC" || die "Compiling patches/$APP.shim.c failed."
    fi
    # ... and empty placeholders for frameworks that only the link step wants to see.
    FWLIST="$SCRIPT_DIR/patches/$APP.stub-frameworks"
    if [ -f "$FWLIST" ]; then
      while IFS= read -r fw || [ -n "$fw" ]; do
        case "$fw" in ""|*[!A-Za-z0-9]*) continue ;; esac
        mkdir -p "$STUBS/$fw.framework"
        printf 'void %s_stub(void) {}\n' "$fw" > "$STUBS/$fw.c"
        clang -dynamiclib -arch x86_64 -mmacosx-version-min=10.15 \
          -install_name "/System/Library/Frameworks/$fw.framework/Versions/A/$fw" \
          -o "$STUBS/$fw.framework/$fw" "$STUBS/$fw.c" || die "Creating the placeholder for the framework $fw failed."
      done < "$FWLIST"
    fi
    sed -e "s|@STUBS_DIR@|$STUBS|g" -e "s|@SHIM_OBJ@|$STUBS/shim.o|g" "$CFG" > "$STUBS/cargo-config.toml"
    CFG="$STUBS/cargo-config.toml"
    mkdir -p .cargo
    if [ -f .cargo/config.toml ]; then
      # The app has its own cargo config (for example an alias): add ours after it.
      if grep -q 'target.x86_64-apple-darwin' .cargo/config.toml; then
        die "$APP already sets x86_64-apple-darwin options in .cargo/config.toml; merge $CFG into it by hand."
      fi
      { cat .cargo/config.toml; echo; cat "$CFG"; } > .cargo/config.toml.new
      mv .cargo/config.toml.new .cargo/config.toml
    else
      cp "$CFG" .cargo/config.toml
    fi
  fi

  # Some apps draw the background of the disk image with resvg (a small tool, installed with cargo).
  if grep -q resvg packaging/macos/package.sh && ! command -v resvg >/dev/null 2>&1; then
    RV="$(sed -n 's/^RESVG_VERSION=\([0-9][0-9.]*\).*/\1/p' packaging/macos/package.sh | head -n 1)"
    say "Installing resvg ${RV:-(latest)} (needed for the disk image; takes a few minutes)"
    if [ -n "$RV" ]; then
      cargo install resvg --locked --version "$RV" || die "Installing resvg failed (see above)."
    else
      cargo install resvg --locked || die "Installing resvg failed (see above)."
    fi
  fi

  trim_package_script packaging/macos/package.sh

  say "Building (the first build of an app takes a long time; warnings are normal)"
  # "bash" in front: some apps (MailCraft) have the script without the executable flag in git.
  # The output is also kept in a log file to tell a failed disk image from a failed build.
  LOG="$WORK/last-build-$APP.log"
  set +e
  CRAFT_I18N_DIR="$WORK/craft-i18n" bash ./packaging/macos/package.sh --arch x86_64 2>&1 | tee "$LOG"
  BUILD_RC="${PIPESTATUS[0]}"
  set -e
  if [ "$BUILD_RC" -ne 0 ]; then
    # The disk image (.dmg) is the very last step, after the app is built, assembled and signed. Some
    # apps need a newer Python for it than Catalina has. The app itself is then finished and usable.
    if grep -q '^==> building .*\.dmg' "$LOG" && ls -d "$APPDIR"/target/macos-package/*.app >/dev/null 2>&1; then
      warn "The app is finished, but the last step (creating the disk image .dmg) failed. The disk image is not needed: use the app from the folder that opens next."
    else
      die_build
    fi
  fi

  OUT="$(find "$APPDIR/target" -maxdepth 3 -type d -name macos-package 2>/dev/null | head -n 1 || true)"
  if [ -z "$OUT" ]; then OUT="$APPDIR"; fi
  DMG="$(find "$APPDIR" -maxdepth 4 -name '*-macos-x86_64.dmg' 2>/dev/null | head -n 1 || true)"
  BUNDLE="$(ls -d "$OUT"/*.app 2>/dev/null | head -n 1 || true)"
  say "Done"
  echo "App folder: $OUT"
  if [ -n "$DMG" ]; then echo "Disk image: $DMG"; fi
  if [ "$MACHINE" = 1 ]; then
    emit "DONE ${BUNDLE:-$OUT}"
  else
    echo "Because the app is not signed by Apple, open it the first time with right-click > Open."
    echo "Note: the app's own update function breaks the Catalina patch. To update, run this script again (or use Craftalina)."
    open "$OUT" || true
  fi
}

# ---- main --------------------------------------------------------------------------------------
[ "$(uname -s)" = "Darwin" ] || die "This script is for macOS."

ACTION="build"
ARG_APP=""
for arg in "$@"; do
  case "$arg" in
    --machine) MACHINE=1 ;;
    --build) ACTION="build" ;;
    --list-apps) ACTION="list-apps" ;;
    --installed) ACTION="installed" ;;
    --check) ACTION="check" ;;
    --install-tools) ACTION="install-tools" ;;
    --install-rust) ACTION="install-rust" ;;
    --info) ACTION="info" ;;
    --copy) ACTION="copy" ;;
    --clean) ACTION="clean" ;;
    -h | --help) sed -n '2,/^# Written for/p' "$SELF" | sed 's/^# \{0,1\}//' | grep -v '^Written for'; exit 0 ;;
    -*) die "unknown option: $arg" ;;
    *) ARG_APP="$arg" ;;
  esac
done

case "$ACTION" in
  list-apps) action_list_apps ;;
  installed) action_installed ;;
  check) action_check ;;
  install-tools) action_install_tools ;;
  install-rust) MACHINE=1; action_install_rust ;;
  info | copy | clean)
    ARG_APP="$(printf '%s' "$ARG_APP" | tr 'A-Z' 'a-z')"
    valid_app "$ARG_APP" || die "unknown app: $ARG_APP"
    "action_${ACTION//-/_}" "$ARG_APP"
    ;;
  build) do_build "$@" ;;
esac
