**Craftalina @VERSION@** (unofficial) is a small window that builds the Craft apps for macOS 10.15 Catalina (Intel) on your own Mac. This download contains **only Craftalina, no Craft app**: the apps are built on your Mac from their public source code.

**Craftalina @VERSION@** (inoffiziell) ist ein kleines Fenster, das die Craft-Apps auf deinem eigenen Mac für macOS 10.15 Catalina (Intel) baut. Dieser Download enthält **nur Craftalina, keine Craft-App**: die Apps werden auf deinem Mac aus ihrem öffentlichen Quellcode gebaut.

### Start / Erster Start
1. Download `Craftalina-@VERSION@-macos10.15-x86_64.zip`, unzip it, drag **Craftalina** into your Applications folder. / Zip laden, entpacken, **Craftalina** in „Programme“ ziehen.
2. The app is not notarized by Apple. Open it the first time with **right-click > Open** (then **Open**). If macOS still blocks it: **System Preferences > Security & Privacy > General > Open Anyway**. / Die App ist nicht von Apple notarisiert. Beim ersten Mal **Rechtsklick > Öffnen** (dann **Öffnen**). Blockiert macOS trotzdem: **Systemeinstellungen > Sicherheit > Allgemein > Trotzdem öffnen**.
3. The window checks what is needed, installs missing parts on a click, and builds the app you pick. / Das Fenster prüft, was fehlt, installiert es per Klick und baut die gewählte App.

The Craft apps need a Metal-capable graphics card. In most virtual machines they build fine but do not start ("No suitable graphics adapter"). / Die Craft-Apps brauchen eine Metal-fähige Grafik. In den meisten virtuellen Maschinen lassen sie sich zwar bauen, starten aber nicht („No suitable graphics adapter").

Craftalina needs an internet connection at start: it downloads the newest build script and patches from this repository. Details and the terminal alternative: see the [README](https://github.com/sudo-mk-sandwich/craft-catalina#readme). / Craftalina braucht beim Start Internet: es lädt das neueste Bauskript und die Patches aus diesem Repository. Details und die Terminal-Alternative: siehe [README](https://github.com/sudo-mk-sandwich/craft-catalina#readme).

SHA-256 of the zip: `@SHA256@`

Not affiliated with or endorsed by the authors of the Craft apps. Problems: [issues](https://github.com/sudo-mk-sandwich/craft-catalina/issues).
