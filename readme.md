# deadlocked

A fork of [avitran0/deadlocked](https://github.com/avitran0/deadlocked) with the unsafe memory-writing features removed.

The following features from the upstream project have been dropped because they write directly to game process memory:

- **No Flash** — overwrote the player pawn's flash alpha value in memory
- **No Smoke** — overwrote the smoke grenade's effect flag in memory
- **Change Smoke Color** — overwrote the smoke grenade's color value in memory
- **FOV Override** — overwrote `m_iDesiredFOV` on the local player's controller

All other functionality (ESP, aimbot, triggerbot, bunnyhop, etc.) is retained.

## Features that write to the input device (`/dev/uinput`)

These features inject synthetic mouse/keyboard events and do **not** touch game memory:

| Feature | What is written |
|---|---|
| **Aimbot** | Relative mouse movement |
| **RCS (Recoil Control System)** | Relative mouse movement |
| **Triggerbot** | Left mouse button press/release |
| **Bunnyhop** | Space bar press/release |

## Features

### Aimbot

Found in the **Aimbot** tab of the overlay GUI.

| Option | Description |
|---|---|
| Hotkeys | One or more keys to hold/toggle the aimbot |
| Mode | Hold or Toggle |
| FOV | Maximum angle from crosshair to snap to a target |
| Smooth | Higher values move the mouse more gradually |
| Start Bullet | Don't activate until this many shots have been fired |
| Targeting Mode | Prioritise closest by FOV or by distance |
| Visibility Check | Only aim at targets with an unobstructed line of sight |
| Flash Check | Don't aim while the local player is flashed |
| **Auto Wall** | Aim through walls when the bullet can deal enough damage (see below) |
| Min Damage | Minimum estimated damage required for Auto Wall to activate (1–500) |
| Bones | Which hit-boxes the aimbot considers |
| Backtrack | Re-use recent position history to hit moving targets |

#### Auto Wall

Auto Wall lets the aimbot aim at enemies that are **behind cover** when your weapon can still deal meaningful damage through the wall.

**Prerequisites**

Auto Wall relies on pre-parsed map geometry (BVH data). The geometry is built automatically on the first run (or after a game update) — you will see log messages like `parsed bvh for de_dust2`. **Let this finish before joining a game.** If no BVH is available for the current map the feature does nothing.

**How to enable**

1. Open the overlay GUI (default: press `Insert` to toggle visibility).
2. Go to the **Aimbot** tab.
3. Expand the **Checks** section.
4. Tick **Auto Wall**.
5. Adjust **Min Damage** to the minimum HP you want the aimbot to be willing to deal through a wall (default: 100). Set it lower to shoot through thicker/denser surfaces; the aimbot will skip a target entirely if no bone can reach `min_damage` (unless the shot would kill outright).

**How it works**

When Auto Wall is enabled the aimbot:

1. Raycasts from the local player's eye position to each bone on the target.
2. Simulates bullet penetration — accounting for wall material (wood, metal, concrete, glass, …), wall thickness, weapon damage, weapon penetration power, armor, and range falloff.
3. Selects the bone that yields the highest estimated damage.
4. Aims at that bone if the damage is ≥ **Min Damage** *or* would kill the target.

Because penetration is calculated per-bone, the aimbot automatically chooses lower limbs or the torso when those are more accessible through a wall than the head.

**Tips**

- High-penetration weapons (AK-47, AWP, SCAR-20, SSG 08, M4A1-S) work best.
- Reduce **Min Damage** if you want the aimbot to shoot even when damage is low.
- The normal **Visibility Check** is automatically bypassed when Auto Wall is active; the penetration calculation takes its place.
- You can enable Auto Wall on a **per-weapon** basis using the **Weapon** tab so that, for example, only the AWP uses it.

### ESP

Found in the **Player** tab of the overlay GUI.  Draws boxes, skeletons, health/armor bars, names, weapon icons, and tags (helmet, defuser, bomb carrier) over enemies and optionally teammates.

### Triggerbot

Found in the **Aimbot** tab of the overlay GUI.  Automatically fires when your crosshair is over an enemy.  Supports configurable delay, scope/flash/velocity checks, and head-only mode.

### RCS (Recoil Control System)

Found in the **Aimbot** tab of the overlay GUI.  Partially counteracts weapon recoil by moving the mouse downward after each shot.

### Bunnyhop

Found in the **Misc** tab of the overlay GUI.  Automatically presses jump at the optimal moment while the hotkey is held.

## Setup

```bash
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh
cargo run --release
```

See [os-setup.md](os-setup.md) for OS-specific setup instructions.
