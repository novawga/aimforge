# Scenario file format

Scenarios are plain [TOML](https://toml.io) files in the `scenarios/` folder (sub-folders are fine).
Every field has a default, so a file only needs the parts it changes. Press **F5** in the menu to reload after editing.

The easiest way to start is to pick any scenario in the menu and click **Export to editable file**. That writes a complete `.toml` with every field filled in.

## How a scenario name is resolved

When a playlist (or you) asks for a scenario by name:

1. **Custom file**: a `.toml` in `scenarios/` whose `name` matches (case-insensitive). Your files always win.
2. **Built-in**: one of the hand-tuned scenarios shipped inside the program (the Voltaic benchmark set and a few classics).
3. **Generated**: built on the fly from the name itself. The generator understands KovaaK's naming conventions:
   - layouts: `1w4ts`, `1wall 6targets`, `ww3t`, `Wide Wall 4 Targets`, `6 Sphere Hipfire`, `Tile Frenzy`
   - types: `Pasu`, `Popcorn`, `B180`/`B180T`/`Bounce 360`, `Pokeball`, `TS`/`TargetSwitch`, `Smoothbot`, `Air`, `Plaza`/`PGTI`, `Close/Midrange Fast/Long Strafes`, `Controlsphere`, `XYZ`, `Skeet`, `Revolving`, `Glider`, `Vertical`, `Floating Heads Timing`, `Reflex`, `Strafe Gallery`
   - modifiers: `small`, `smaller`, `extra small`, `30% smaller`, `25% larger`, `thin`, `extra thin`, `easy`, `hard`, `slow`, `85%` (speed), `Iron`..`Grandmaster`, `invincible`, `click`, `track`, `dodge`, `static`, `shrink`, `regen`, `HS`, `30s`/`60s`

Generated scenarios are deterministic: the same name always gives the same scenario, so high scores stay comparable.

## Coordinates

Units are **metres**. +Y is up. The player starts at `player.position` looking down **+Z** (yaw 0). Positive yaw turns **right**. A typical wall sits 18 m away (`z = 18`).

## Reference

```toml
name = "Untitled"
category = "Clicking"   # Clicking | Tracking | Switching | Timing | Strafe  (menu filter + colour only)
author = ""
description = ""
duration = 60.0         # seconds

[player]
position = [0.0, 0.0, 0.0]   # feet
yaw = 0.0
pitch = 0.0
eye_height = 1.6
can_move = true          # WASD + Space
move_speed = 6.0
jump_velocity = 5.0
gravity = 15.0
forced_strafe = 0.0      # > 0: player is auto-strafed left/right at this speed (strafe aim)
forced_strafe_min = 0.4  # seconds between direction swaps
forced_strafe_max = 1.0

[weapon]
mode = "Semi"            # Semi = one shot per click, Auto = fires while held (tracking)
fire_rate = 0.0          # shots/sec. Semi: click-rate cap (0 = none). Auto: default 15 if 0
damage = 100.0
headshot_multiplier = 1.0
magazine = 0             # 0 = infinite
reload_time = 1.0

[arena]                  # the room; targets and player stay inside
min = [-14.0, 0.0, -3.0]
max = [14.0, 12.0, 19.5]
grid = 1.0               # grid line spacing on walls/floor (0 = none)

[targets]
shape = "Sphere"         # Sphere | Bot (capsule body + head)
radius = 0.3
height = 1.8             # bots: total height
head_radius = 0.0        # bots: 0 = no separate head
head_only = false        # bots: only the head takes damage
health = 100.0
invincible = false       # infinite health; score with per_damage (tracking)
regen_rate = 0.0         # HP/s after regen_delay seconds without damage
regen_delay = 1.0
max_alive = 3            # targets on screen at once
total = 0                # total spawns per run (0 = unlimited); run ends when all are cleared
respawn_delay = 0.0
lifetime = 0.0           # > 0: target disappears after this many seconds (click timing)
shrink_rate = 0.0        # radius lost per second
min_radius = 0.05
# color = [255, 0, 0]    # optional fixed colour (otherwise your Settings colour)

[spawn]
shape = "Box"            # Box | Arc | Grid
box_min = [-5.0, 1.5, 18.0]   # Box: uniform random inside this box
box_max = [5.0, 6.0, 18.0]
yaw_min = -45.0          # Arc: around the player's eye, degrees relative to start yaw
yaw_max = 45.0
pitch_min = -5.0
pitch_max = 20.0
dist_min = 10.0
dist_max = 16.0
grid_center = [0.0, 3.5, 18.0] # Grid: fixed slots facing the player (Tile Frenzy)
grid_cols = 3
grid_rows = 3
grid_spacing = 1.6
min_separation = 1.0     # metres between a new target and live ones

[movement]
pattern = "Static"       # Static | Bounce | Strafe | Air | Popcorn | Orbit | Glider
speed_min = 3.0          # m/s (Orbit: tangential m/s)
speed_max = 5.0
change_min = 0.4         # seconds between direction changes
change_max = 1.2
accel = 0.0              # m/s² towards the new velocity; 0 = instant (snappy), 8-15 = smooth
axes = [true, true, false]   # Bounce: which of x, y, z it may move along
gravity = 12.0           # Air/Popcorn/Strafe jumps. Glider: fall speed in m/s
jump_min = 4.0           # jump velocity range
jump_max = 7.0
jump_chance = 0.0        # Strafe: chance to jump at each direction change
blink_min = 0.0          # > 0: teleport every blink_min..blink_max seconds
blink_max = 0.0
min_player_dist = 5.0    # targets never come closer than this to your eye
# bounds_min = [-8.0, 1.0, 11.0]   # movement box; default = spawn box (Box spawns) or arena
# bounds_max = [8.0, 7.0, 13.0]

[scoring]
per_kill = 100.0
per_hit = 0.0
per_damage = 0.0         # tracking: 1.0 = score equals damage dealt
per_miss = 0.0           # subtracted per missed shot
per_expire = 0.0         # subtracted when a timed target expires
```

### Movement patterns

| Pattern | What it does | Good for |
|---|---|---|
| `Static` | Doesn't move | static clicking, pokeball |
| `Bounce` | Straight lines, new random direction every change interval, bounces off bounds. With `accel` it curves smoothly | pasu, B180, smoothbot, XYZ, switching |
| `Strafe` | ADAD left/right relative to you, optional jumps | close/long strafes, plaza |
| `Air` | Strafe + gravity + always jumping on landing | Air scenarios |
| `Popcorn` | Hops off the floor with random sideways velocity | popcorn |
| `Orbit` | Circles around you, reversing direction | revolving, psalmTS |
| `Glider` | Slow fall with drift, respawns at the top | glider scenarios |
