# deadlocked

A fork of [avitran0/deadlocked](https://github.com/avitran0/deadlocked).

## Features that write to game process memory

The following active features use `process_vm_writev` to write directly into the CS2 process:

| Feature | What is written |
|---|---|
| **FOV Override** (`misc.desired_fov`) | `m_iDesiredFOV` on the local player's controller |

## Features that write only to the input device (`/dev/uinput`)

These features inject synthetic mouse/keyboard events and do **not** touch game memory:

| Feature | What is written |
|---|---|
| **Aimbot** | Relative mouse movement |
| **RCS (Recoil Control System)** | Relative mouse movement |
| **Triggerbot** | Left mouse button press/release |
| **Bunnyhop** | Space bar press/release |

## Removed upstream features

The following features from the upstream project were dropped because they wrote directly to game process memory:

- **No Flash** — overwrote the player pawn's flash alpha value in memory
- **No Smoke** — overwrote the smoke grenade's effect flag in memory
- **Change Smoke Color** — overwrote the smoke grenade's color value in memory

All other functionality (ESP, aimbot, triggerbot, bunnyhop, etc.) is retained.

## Setup

```bash
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh
cargo run --release
```

See [os-setup.md](os-setup.md) for OS-specific setup instructions.
