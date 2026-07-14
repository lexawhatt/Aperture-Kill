# Portals

Build and run:

```sh
cargo run
```

On Linux, audio needs ALSA because `rodio` uses `alsa-sys`. If Cargo fails with
`Package 'alsa' ... not found` or `alsa.pc needs to be installed`, install the
ALSA development package:

Ubuntu/Debian:

```sh
sudo apt install libasound2-dev pkg-config
```

Fedora:

```sh
sudo dnf install alsa-lib-devel pkgconf-pkg-config
```

Arch:

```sh
sudo pacman -S alsa-lib pkgconf
```
