# deadlocked

A fork of [avitran0/deadlocked](https://github.com/avitran0/deadlocked) with the unsafe memory-writing features removed.

The following features from the upstream project have been dropped because they write directly to game process memory:

- **No Flash** — overwrote the player pawn's flash alpha value in memory
- **No Smoke** — overwrote the smoke grenade's effect flag in memory
- **Change Smoke Color** — overwrote the smoke grenade's color value in memory
- **FOV Override** — overwrote `m_iDesiredFOV` on the local player's controller

All other functionality (ESP, aimbot, triggerbot, bunnyhop, etc.) is retained.

## Features that write to the input device (`/dev/uinput` or `xdotool`)

These features inject synthetic mouse/keyboard events and do **not** touch game memory:

| Feature | Mechanism | What is written |
|---|---|---|
| **Aimbot** | `/dev/uinput` | Relative mouse movement |
| **RCS (Recoil Control System)** | `/dev/uinput` | Relative mouse movement |
| **Triggerbot** | `/dev/uinput` | Left mouse button press/release |
| **Bunnyhop (SEMI BROKEN ATM)** | `/dev/uinput` | Space bar press/release |
| **Anti-AFK** | `xdotool` subprocess | Small relative mouse movement; optionally a random WASD key press (walk bot) |

## Setup

```bash
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh
cargo run --release
```

See [os-setup.md](os-setup.md) for OS-specific setup instructions.
