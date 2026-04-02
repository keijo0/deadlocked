# deadlocked

A CS2 game overlay tool written in Rust for Linux, forked from
[avitran0/deadlocked](https://github.com/avitran0/deadlocked) with all
unsafe memory-writing features removed.

> **Linux only.** X11 session required (Wayland is run through XWayland).

---

## Features

| Feature | Description |
|---|---|
| **ESP** | Draw boxes, health bars, names, and distance labels on players |
| **Aimbot** | Mouse-movement-based aim assist via `/dev/uinput` |
| **RCS** | Recoil control system via `/dev/uinput` |
| **Triggerbot** | Automatically fires when crosshair is on an enemy |
| **Bunnyhop** | Auto-jump (partially working) |
| **Anti-AFK** | Periodic mouse jitter and optional random WASD walk |

The following features from the upstream project have been **dropped** because
they write directly to game process memory:

- **No Flash** — overwrote the player pawn's flash alpha value in memory
- **No Smoke** — overwrote the smoke grenade's effect flag in memory
- **Change Smoke Color** — overwrote the smoke grenade's color value in memory
- **FOV Override** — overwrote `m_iDesiredFOV` on the local player's controller

---

## Requirements

| Dependency | Purpose |
|---|---|
| `git` | Cloning the repository |
| `rustup` / `cargo` | Building the project |
| `sudo` | Running `setup.sh` (udev rules, group membership) |
| `xdotool` | Anti-AFK feature |
| `iptables` | Network-level filtering (optional, used for server picking) |
| X11 libraries | Overlay window rendering |

---

## Quick start

```bash
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh          # creates udev rules and adds you to the uinput group
# Log out and log back in so the new group takes effect
cargo run --release
```

See [os-setup.md](os-setup.md) for per-distro installation instructions
(Arch Linux, Debian/Ubuntu, Fedora, openSUSE, Fedora Atomic, NixOS).

### Updating

```bash
./run.sh   # pulls latest changes and re-runs
```

---

## Command-line flags

| Flag | Description |
|---|---|
| `--force-reparse` | Force re-parsing of CS2 map files even if cache is valid |
| `--local-s2v` | Use the system-installed `Source2Viewer` binary instead of the one in `resources/` |

---

## Memory safety guarantee

No code in this repository writes to the game process memory.  There are two
mechanisms used to **read** game memory, and both are strictly read-only:

1. **`process_vm_readv`** — the Linux `process_vm_readv(2)` syscall is used for
   the bulk of reads (`read`, `read_or_zeroed`, `read_vec`, `read_typed_vec`).
   This is a read-only syscall; its write counterpart `process_vm_writev` is
   never called anywhere in the codebase.

2. **`/proc/{pid}/mem` via `ReadOnlyMem`** — the file `/proc/{pid}/mem` is
   opened with `OpenOptions::new().read(true)` and wrapped in a private
   `ReadOnlyMem` newtype (`src/os/process.rs`) that exposes **only** `read_at`.
   Because the inner `File` is private and `.write(true)` is never passed to
   `OpenOptions`, it is a **compile-time error** to call any write operation on
   this handle.  This path is used by `read_bytes` (and therefore `dump_module`
   and `scan`).

---

## Input injection (`/dev/uinput` and `xdotool`)

These features inject synthetic mouse/keyboard events and do **not** touch game memory:

| Feature | Mechanism | What is written |
|---|---|---|
| **Aimbot** | `/dev/uinput` | Relative mouse movement |
| **RCS (Recoil Control System)** | `/dev/uinput` | Relative mouse movement |
| **Triggerbot** | `/dev/uinput` | Left mouse button press/release |
| **Bunnyhop (SEMI BROKEN ATM)** | `/dev/uinput` | Space bar press/release |
| **Anti-AFK** | `xdotool` subprocess | Small relative mouse movement; optionally a random WASD key press (walk bot) |

---

## Troubleshooting

**`/dev/uinput` permission denied**  
Run `./setup.sh` and then fully log out and back in (or reboot) to pick up the
new `uinput` group membership.

**Overlay window has blur on Hyprland**  
`setup.sh` automatically adds the required `windowrule` to
`~/.config/hypr/hyprland.conf` when Hyprland is detected.

**Map parsing fails**  
Run with `--local-s2v` to use the system-installed `Source2Viewer`:

```bash
cargo run --release -- --local-s2v
```

**Downloaded as a ZIP instead of cloning**  
The update script (`run.sh`) requires a proper git repository.  Delete the
extracted folder and clone instead:

```bash
git clone --recursive https://github.com/keijo0/deadlocked
```
