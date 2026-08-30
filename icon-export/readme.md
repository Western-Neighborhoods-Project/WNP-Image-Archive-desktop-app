# Archive app icon — export kit (mark 2i)

Flat mark: three rounded tiles (sage #ccdbb2, sand #ebddc5, sage #7a8a5e) + terracotta lens (#c67139) on cream (#f5ead8).

## Why two paths
On macOS 26 (Tahoe), apps that ship only a legacy .icns get the system's default treatment (shrunk onto a generated glass backdrop) in inactive/tinted/clear modes. The only sanctioned way to control how the icon renders on Tahoe is to ship an Icon Composer (.icon) asset compiled into Assets.car — alongside the .icns for older macOS.

## 1. Tahoe (.icon via Icon Composer)
1. Open Apple's Icon Composer (ships with Xcode 26 tools).
2. New macOS icon; add the three layers in order:
   - layer-1-background-1024.png (background)
   - layer-2-tiles-1024.png
   - layer-3-lens-1024.png
3. Keep it flat: on every layer turn OFF specular highlight, set blur/translucency to none, no shadow between layers (or minimum if forced). Icon Composer applies the squircle mask itself — the layers are full-bleed squares.
4. Save as AppIcon.icon.
5. Wire into Tauri (no native support yet — tauri-apps/tauri#14207):
   - npx tauri-liquid-icon --icon ./AppIcon.icon --name AppIcon
   (compiles via actool to Assets.car, updates Info.plist CFBundleIconFile and tauri.conf.json bundle resources; needs Xcode CLT)
   - Or manually: actool → Assets.car into Contents/Resources, set CFBundleIconName in Info.plist.

## 2. Pre-Tahoe (.icns)
- Run: npm run tauri icon ./icon-export/app-icon-legacy-1024.png
  (this file already has the classic 824px squircle-with-margin geometry; tauri icon generates the .icns + all sizes)
- Ship BOTH: keep the .icns in the bundle for macOS ≤ 15; Tahoe prefers Assets.car when present.

app-icon-full-bleed-1024.png is the flat composite if you ever need a single square source.
