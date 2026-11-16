**Craftalina {{VERSION}}** (unofficial) is a small window that builds the Craft apps for macOS 10.15 Catalina (Intel) on your own Mac. This download contains **only Craftalina, no Craft app**: the apps are built on your Mac from their public source code.

### First start
1. Download `Craftalina-{{VERSION}}-macos10.15-x86_64.zip`, unzip it and drag **Craftalina** into your Applications folder.
2. The app is not notarized by Apple. Open it the first time with **right-click > Open**. If macOS still blocks it: **System Preferences > Security & Privacy > General > Open Anyway**.
3. The window checks what is needed, installs missing parts on a click and builds the app you pick.

Craftalina needs an internet connection at start: it downloads the newest build script and patches from this repository. The Craft apps need a Metal-capable graphics card; in most virtual machines they build but do not start. Details and the terminal alternative: [README](https://github.com/sudo-mk-sandwich/craft-catalina#readme).

SHA-256 of the zip: `{{SHA256}}`

Not affiliated with or endorsed by the authors of the Craft apps. Problems: [issues](https://github.com/sudo-mk-sandwich/craft-catalina/issues).
