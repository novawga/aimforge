# AimForge

A KovaaK's-style 3D aim trainer written from scratch in Rust. Flicking, tracking, target switching, click timing and strafe aim, all driven by plain-text scenario files. It plays KovaaK's playlists directly.

- **Updates itself** from GitHub releases, like a normal launcher.
- **Leaderboards and replays**: every run is recorded (about 100 KB per minute) and can be watched back in first person with pause, seek and 0.25×–2× speed. Each scenario has a global board (best run per player, with score, accuracy, kills, sens and a Watch button) plus your own top runs. The results screen shows your rank straight away.
- **Accounts**: sign in (or play as a guest) on launch; your name is reserved and works on any PC. Startup shows an animated loading range while scenarios, sounds and shaders load.
- **Every playlist scenario is playable.** Drop KovaaK's playlists (`.json` or legacy `.plo`) into `playlists/`. Scenarios you've written or that ship built-in are used as-is; anything else is generated from its name (`1w4ts reload 30% smaller`, `B180T Voltaic Easy`, `Close Fast Strafes Invincible`...). This build was tested against 1,096 unique scenario names from 189 community playlists.
- **Your sensitivity, exactly.** Enter your CS2/Apex/KovaaK's, Valorant or Overwatch sens (or cm/360 directly) plus DPI. Mouse input is raw and processed in order, so a click lands where your crosshair was at that instant, even mid-frame.
- **Built-in Voltaic benchmark set**: Pasu, B180, Popcorn, ww3t, 1w4ts, Smoothbot, Air Angelic 4, PGTI, FuglaaXYZ, Ground Plaza, Air, patTS, voxTS, kinTS, B180T, psalmTS, plus Tile Frenzy, 1wall 6targets small, Close Long Strafes, Floating Heads Timing, Strafe Gallery and Pokeball Frenzy.
- **KovaaK's-style flow**: scenario browser with search, category filters and high scores; playlists with "Next (Enter)"; countdown; post-run stats with score history graph; R to restart; Esc to pause.
- **Stats you own**: every run goes to `stats/history.json` plus a KovaaK's-style `stats/runs/<Scenario> - Challenge - <date> Stats.csv` with a kill-by-kill log.
- **Valorant-ready**: pick Valorant (or CS2, Apex, Overwatch 2, KovaaK's, cm/360) and type the sens you use there; a table shows the same aim in every other game, with eDPI and cm/360. Fine-tune with ±1% buttons, even from the pause menu mid-practice.
- **Valorant crosshairs**: the crosshair uses Valorant's model (inner/outer lines, centre dot, outlines). Paste a Valorant crosshair profile code to import it, or copy yours back out.
- **Maps**: 17 procedural skies drawn on the GPU (clear day with clouds, golden hour, sunset, synthwave, aurora, starry night, nebula, deep space...), 11 floor/wall patterns (grid, neon grid, tiles, hex, marble, planks, concrete...), three arena layouts (room, open roof, open field), fog, and 16 one-click map presets. Target colour plus four shading styles (matte, glossy, flat, glow).
- **Sound**: synthesized hit sounds (Tick, Ting, Pop, Click, Bubble, Wood, Thump) and kill sounds (Ding, Chime, Sparkle, Shatter, Coin, Zap, Kick), countdown/finish/high-score cues and soft menu clicks. Drop your own `.wav`/`.ogg` files into `sounds/` to add more.
- **Menu themes**: six built-in themes (Graphite, Valorant, Classic Grey, Midnight, Nord, Daylight) that restyle the menus, HUD and 3D room. Themes are small `.toml` files in `themes/`: import one from the clipboard, copy yours to share, or save and edit your own.
- **Customisable**: target colour, FOV, hit/kill sounds, hitmarker, health bars, auto-fire for tracking, V-Sync, MSAA.

## Download

Grab **`aimforge-windows-x86_64.exe`** from the [latest release](https://github.com/novawga/aimforge/releases/latest), put it in its own folder (it keeps its settings, stats and playlists next to itself), and run it. Windows may show a SmartScreen warning because the exe isn't code-signed: click *More info → Run anyway*.

On first launch it downloads the community playlists from [riddbtw/kovaaks-playlists](https://github.com/riddbtw/kovaaks-playlists).

### Updates

AimForge checks this repo's releases every time it starts. When there's a newer version it downloads it in the background, checks its SHA-256, swaps it in and restarts. You keep your settings, stats and files. Turn this off in *Settings → Files & controls*.

To publish a new version: bump `version` in `Cargo.toml` and push to `main`. The `release` workflow notices the new version, builds Windows, macOS and Linux binaries and attaches them to a GitHub release. Every installed copy picks it up on its next launch.

### Online leaderboards (Supabase)

Leaderboards use a free [Supabase](https://supabase.com) project:

1. Create a project, open **SQL Editor**, paste `supabase/setup.sql` and click **Run**. Then do the same with `supabase/accounts.sql` (player accounts for the sign-in screen).
2. In **Project Settings → API Keys**, copy the **Project URL** and the **publishable** (or legacy anon) key.
3. Put them in `src/leaderboard.rs` (`DEFAULT_URL` / `DEFAULT_KEY`) and push a new version, or set `board_url` / `board_key` in `settings.toml` for just your copy.

The publishable key is meant to be public: row-level security only lets clients read scores and add new ones, and the accounts table can only be reached through the sign-in functions (passwords are stored as bcrypt hashes). Scores aren't verified, so this suits friends and communities, not prize competitions.

## Build from source

1. Install Rust from <https://rustup.rs> (on Windows, accept the Visual Studio C++ build tools prompt).
2. In this folder:

   ```
   cargo run --release
   ```

The release binary ends up in `target/release/` (`aimforge.exe` on Windows). To make a portable copy, put the binary in a folder next to `scenarios/` and `playlists/`. It reads and writes its files beside itself.

**Linux** also needs: `sudo apt install libasound2-dev libx11-dev libxi-dev libgl1-mesa-dev`
(or build without sound: `cargo run --release --no-default-features`).

Copies run from `target/release` (a cargo build) don't self-update; use `git pull` and rebuild instead.

Jump straight into a scenario: `aimforge --play "Pasu Voltaic"`, and pick a map preset with `--map "Synthwave"` (handy for desktop shortcuts).

## Controls

| | |
|---|---|
| Mouse / Left click | Aim / shoot (hold in tracking scenarios) |
| WASD, Space | Move, jump |
| R | Restart scenario |
| Esc | Pause / back |
| Enter | Next scenario in playlist (results screen) |
| F5 | Reload scenario + playlist files (menu) |
| F11 | Toggle fullscreen |
| Double-click a row | Play that scenario |

## Folders

```
scenarios/   your .toml scenario files (sub-folders OK)  -> see SCENARIO_FORMAT.md
playlists/   KovaaK's playlists: .json or .plo (sub-folders become groups)
themes/      menu colour themes (.toml), see themes/Example - Forest.toml
sounds/      your own hit/kill sounds (.wav / .ogg)
stats/       history.json + per-run CSV files
settings.toml
assets/      optional font.ttf / font-bold.ttf to override the UI font
```

Set `AIMFORGE_HOME=/some/folder` to keep all of these somewhere else.

## Making scenarios

Select any scenario and click **Export to editable file**. You get a complete `.toml` in `scenarios/`. Edit it, press F5, and your version takes over that name everywhere, including in playlists. `scenarios/examples/My First Scenario.toml` is a small commented example, and `SCENARIO_FORMAT.md` documents every field.

Playlists you build with **Add to My Playlist** are saved in KovaaK's JSON format, so they work in both programs.

## Sensitivity notes

- Scales: Valorant = 0.07°/count, CS2/Apex/KovaaK's = 0.022, Overwatch 2 = 0.0066, or enter cm/360 directly. Switching games keeps your physical cm/360, so Valorant 0.4 at 800 DPI becomes CS2 1.273.
- FOV is horizontal at 16:9. Valorant, Overwatch 2 and KovaaK's use 103; CS2 is 106.26. There are one-click presets on the Sensitivity page.
- Windows reads raw mouse counts (`WM_INPUT`), unaffected by Windows pointer speed or Enhance Pointer Precision.
- Linux/X11 delivers each raw mouse report several times (once per input device); AimForge detects and removes the duplicates automatically. Debug with `AIMFORGE_DEBUG_INPUT=1`.

## Development

```
cargo test --no-default-features                       # unit + simulation tests
PLAYLIST_DIR=playlists cargo test --no-default-features -- --include-ignored --nocapture
                                                       # generate + simulate every scenario in your playlists
```

Code map: `app.rs` state and run control · `screens/` menu, settings, in-game HUD, results · `map.rs` sky + surface shaders and map presets · `audio.rs` sound synthesis · `theme.rs` menu themes · `crosshair.rs` Valorant crosshair model + codes · `game.rs` simulation (player, weapons, target movement, hit detection, scoring) · `synth.rs` name-to-scenario generator · `scenario.rs` file format + library · `playlist.rs` JSON/GVAS import · `render.rs` 3D + crosshair · `replay.rs` run recording + playback format · `leaderboard.rs` Supabase client · `input.rs` ordered raw mouse input · `ui.rs` immediate-mode widgets.

## License

MIT (see `LICENSE`). Community playlists are downloaded from their own repository and belong to their authors.
