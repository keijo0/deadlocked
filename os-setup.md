# OS-specific installation guide

This page covers everything you need to install on each supported Linux
distribution before you can build and run **deadlocked**.

### Core cheat

All distributions require:
- `git` — to clone the repository
- `rustup` / `cargo` — to build the Rust project
- `sudo` — used by `setup.sh` to create udev rules and manage group membership
- `xdotool` — used by the Anti-AFK feature
- X11 / OpenGL libraries — required by the overlay window

### Server picker (optional)

The server picker lets you block CS2 matchmaking relays by continent.
It requires two extra tools — **only install these if you plan to use that feature**:
- `curl` — fetches the relay list from the Steam API
- `iptables` — blocks / unblocks relay IPs at the network level (`sudo` is reused from above)

---

## Arch Linux / Manjaro

```bash
# 1. Install core dependencies
sudo pacman -Syu --needed git sudo xdotool \
    libx11 libxcursor libxkbcommon libxi mesa

# 1a. (Optional) Server picker dependencies
sudo pacman -S --needed curl iptables

# 2. Install Rust (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Clone and set up
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh

# 4. Log out and back in (or reboot) so group changes take effect

# 5. Build and run
cargo run --release
```

---

## Debian / Ubuntu / Linux Mint

```bash
# 1. Install core dependencies
sudo apt update
sudo apt install -y git sudo xdotool \
    libx11-dev libxcursor-dev libxkbcommon-dev libxi-dev \
    libgl1-mesa-dev pkg-config build-essential

# 1a. (Optional) Server picker dependencies
sudo apt install -y curl iptables

# 2. Install Rust (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Clone and set up
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh

# 4. Log out and back in (or reboot) so group changes take effect

# 5. Build and run
cargo run --release
```

---

## Fedora Workstation

```bash
# 1. Install core dependencies
sudo dnf install -y git sudo xdotool \
    libX11-devel libXcursor-devel libxkbcommon-devel libXi-devel \
    mesa-libGL-devel pkg-config gcc

# 1a. (Optional) Server picker dependencies
sudo dnf install -y curl iptables

# 2. Install Rust (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Clone and set up
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh

# 4. Log out and back in (or reboot) so group changes take effect

# 5. Build and run
cargo run --release
```

---

## openSUSE Tumbleweed / Leap

```bash
# 1. Install core dependencies
sudo zypper install -y git sudo xdotool \
    libX11-devel libXcursor-devel libxkbcommon-devel libXi-devel \
    Mesa-libGL-devel pkg-config gcc

# 1a. (Optional) Server picker dependencies
sudo zypper install -y curl iptables

# 2. Install Rust (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Clone and set up
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh

# 4. Log out and back in (or reboot) so group changes take effect

# 5. Build and run
cargo run --release
```

---

## Fedora Atomic (Silverblue / Kinoite)

The root filesystem is immutable, so package installs use `rpm-ostree` and
require a reboot, or you can layer them into a toolbox.

```bash
# 1. Add yourself to the input group (immutable /usr/lib/group workaround)
grep -E '^input:' /usr/lib/group | sudo tee -a /etc/group
sudo usermod -aG input "$USER"

# 2. Install core layered packages (requires reboot)
rpm-ostree install xdotool
# 2a. (Optional) Also install server picker dependencies
rpm-ostree install curl iptables
# Reboot to apply the layered packages:
systemctl reboot

# 3. After reboot — install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 4. Clone and set up
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
./setup.sh

# 5. Log out and back in (or reboot) so uinput group changes take effect

# 6. Build and run
cargo run --release
```

---

## NixOS

Add `"input"` to your user's `extraGroups` in `configuration.nix`:

```nix
users.users.yourname = {
  isNormalUser = true;
  extraGroups = [ "wheel" "input" ];
};
```

Then rebuild and reboot:

```bash
sudo nixos-rebuild switch
sudo reboot
```

After reboot, the Nix dev shell (via `direnv`) provides all build dependencies
automatically — no manual package installation needed:

```bash
git clone --recursive https://github.com/keijo0/deadlocked
cd deadlocked
direnv allow        # activates the Nix dev shell defined in flake.nix
cargo run --release
```

If map parsing fails, use the `Source2Viewer` provided by the Nix dev shell
instead of the bundled binary:

```bash
cargo run --release -- --local-s2v
```

Everything else is configured in `flake.nix` and `nix/shell.nix`.

> **Server picker on NixOS:** `curl` and `iptables` are not included in the Nix
> dev shell by default.  To use the server picker, either add them to your system
> `configuration.nix` (`environment.systemPackages`) or install them temporarily
> with `nix-shell -p curl iptables`.

---

## After installation — common steps

### Verify `/dev/uinput` access

```bash
ls -l /dev/uinput
# Should show group "uinput" and that your user is in that group:
groups | grep uinput
```

### Run the tool

```bash
cd deadlocked
cargo run --release
```

### Update to the latest version

```bash
cd deadlocked
./run.sh   # pulls latest changes and re-runs
```

---

## Troubleshooting

**Permission denied on `/dev/uinput`**  
Make sure `./setup.sh` ran successfully and that you have logged out and back in
(or rebooted) since running it.

**`xdotool` not found**  
Install it with your package manager (see the relevant section above).  The
Anti-AFK feature will not work without it.

**`iptables` or `curl` not found**  
Install them with your package manager (see the optional server picker step in
the relevant section above).  Both are only needed for the server-picker feature.

**Overlay window has blur on Hyprland**  
`setup.sh` automatically adds the required `windowrule` to
`~/.config/hypr/hyprland.conf` when Hyprland is detected.  If you skipped
`setup.sh`, add this line manually:

```
windowrule = no_blur 1, match:title ^(deadlocked_overlay)$
```

**Map parsing fails**  
Run with `--local-s2v` to use the system-installed `Source2Viewer`:

```bash
cargo run --release -- --local-s2v
```
