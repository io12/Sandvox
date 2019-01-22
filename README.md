[![Build Status](https://travis-ci.org/io12/Sandvox.svg?branch=master)](https://travis-ci.org/io12/Sandvox)
![](https://img.shields.io/crates/v/sandvox.svg)

# Sandvox

The 3D voxel falling-sand game

# Building

Run `cargo build --release`. Without `--release`, the game is unplayably slow.

# TODO

- Client
  - [ ] Proper player hitbox
  - [ ] Running
  - [x] Air control
  - [ ] Adjustable brush size
  - [ ] Debug HUD
  - [ ] Color variation
  - [ ] Pressure
  - [ ] Better shading
  - [ ] Better sandfall logic (resolve the statefulness issue)
  - Choice of materials
    - [ ] Wall
    - [ ] Wood
    - [ ] Water
    - [ ] Sand
    - [ ] Ice
    - [ ] Fire
    - [ ] Lava
    - [ ] Stone
    - [ ] Oil
    - [ ] Acid
    - [ ] Dust
  - [ ] 3D environment outside the game area
  - [ ] Realistic lighting/shadows
  - [ ] Realistic physics
  - [ ] Saving
  - [ ] UI
  - [ ] Stereo sound effects
  - [ ] Screenshotting
  - [ ] Color shade when player is inside a material
- Server
  - [ ] Upload/rate worlds
  - [ ] Multiplayer
- Deployment
  - [x] crates.io
  - [ ] Add documentation on docs.rs
  - [ ] Logo
  - [ ] Linux app (.desktop file, icon)
  - [ ] Linux AppImage
  - [ ] DEB
  - [ ] RPM
  - [ ] Ubuntu PPA
  - [ ] AUR package
  - [ ] Windows executable icon
  - [ ] Mac app (not just a raw executable)
  - [ ] Homebrew-cask
  - [ ] Website
  - [ ] Android app
