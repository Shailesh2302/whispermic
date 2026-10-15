# Icons
Place generated icons here before `npx tauri build`:

```bash
cd apps/desktop
# put a 1024x1024 src icon at ./icon.png then:
npx tauri icon icon.png
# generates: 32x32.png 128x128.png icon.icns icon.ico ...
```

`tauri.conf.json` already references these names. `cargo check` / `npx tauri dev`
work without them; only `npx tauri build` needs them.
