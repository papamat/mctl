# LSF Mission Control

Development branch: `lsf-mission-control`. This profile uses the upstream source/engine/UI architecture and
CelesTrak GP propagation. It does not claim to receive LSF spacecraft telemetry.

## Verified catalog, 2026-11-05

| Spacecraft | Official catalog | State / treatment |
|---|---:|---|
| PHASMA-DIRAC | 66681 | Operational; enabled, CelesTrak GP epoch 2026-11-04T17:42:45.046368 UTC |
| PHASMA-LAMARR | 66673 | Operational; enabled, GP epoch 2026-11-03T10:30:13.595040 UTC |
| SIDLOC-01 | 60239 | Inactive hosted payload on ARIANE 6 R/B (2024-128E); orbit only, labeled INACTIVE |
| SIDLOC PQ-03/04/05/06 | 101092 / 101097 / 101103 / 101096 | Enabled with `[ID?]`; provisional identity crosswalk detailed below |
| NIS-PQ / QUBIK-5 | 101093 | Enabled; CelesTrak names it NIS-PQ, 2026-227CF |
| PICOBUS v2 on OSSIE-D | 100933 | Hosted deployer; follows OSSIE D, 2026-227BC |
| QUBIK-1/2 | None | Launch failure, excluded |
| QUBIK-3/4 | Historical | Decayed, excluded |

Sources: [LSF mission inventory](https://www.libre.space/missions/),
[QUBIK status](https://www.libre.space/projects/qubik/),
[SatNOGS DIRAC](https://db.satnogs.org/satellite/GPEA-4779-0511-8218-9913/), and direct
[CelesTrak DIRAC GP](https://celestrak.org/NORAD/elements/gp.php?CATNR=66681&FORMAT=JSON),
[LAMARR GP](https://celestrak.org/NORAD/elements/gp.php?CATNR=66673&FORMAT=JSON),
[ARIANE 6 R/B GP](https://celestrak.org/NORAD/elements/gp.php?CATNR=60239&FORMAT=JSON).
The R/B element epoch was 2026-11-04T19:45:11.581344 UTC. Catalog/name/epoch were checked together.
Elements are fetched at runtime, not frozen at these epochs; source age/availability remains visible.
Do not add provisional numbers as `norad`. Before enabling another vehicle, verify its official identity,
mission state and a recent matching GP record. Unlaunched missions are not enabled; a labeled synthetic-orbit
mode remains future work. Clock warp/replay is not a substitute for a verified orbit.

## Fedora development

Install Rust 1.95.0 (including rustfmt/clippy) and a C compiler. The workstation setup used a workspace-local
Rust installation without changing shell profiles. In this workspace:

```sh
export CARGO_HOME='/home/papamat/Documents/mctl LSF missions/.toolchain/cargo'
export RUSTUP_HOME='/home/papamat/Documents/mctl LSF missions/.toolchain/rustup'
export PATH="$CARGO_HOME/bin:$PATH"
cd '/home/papamat/Documents/mctl LSF missions/mctl'
cargo build --release --locked -p mctl
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
scripts/lsf-display
```

The launcher uses `config/lsf/mctl/config.toml` as an isolated configuration and does not overwrite the user's
normal configuration. Reorder its `[[vehicle]]` blocks to change satellite order; remove a block to exclude it.
Recording is disabled to reduce SD-card writes. The observer is `LSF LAB`, using operator-supplied coordinates
37.999040378659245 N, 23.706396222021784 E (2026-11-05). Altitude was not supplied and remains an explicit
temporary assumption of 0 m; update `observer.fallback.alt_m` when the measured altitude is available.

`config/lsf/themes/green.toml` independently controls the four CRT effects. Scanlines and mild curvature are
on; bloom and persistence are off by default. Enable bloom for subtle glow only after measuring the Pi.
Cheap flat widget drawing remains enabled even when post-processing is selected.

Launcher environment settings:

| Variable | Default | Meaning |
|---|---|---|
| `MCTL_BIN` | `target/release/mctl` | Executable path |
| `LSF_CONFIG_HOME` | repository `config/lsf` | Directory containing `mctl/config.toml` |
| `LSF_STYLE` | `dense` | `dense`, `pretty`, `mixed` visualization groups |
| `LSF_ORDER` | `sequential` | `sequential`, `random`, `vehicle` |
| `LSF_DWELL` | `12` | Base seconds: dense pages multiply by 1.5 (18 seconds) |
| `LSF_MAX_FPS` | `10` | Frame-rate ceiling |
| `LSF_EFFECTS` | `1` | `0` disables post-processing entirely |

Pretty/mixed modes retain upstream art/interludes; individual scene selection is not implemented yet.
Pretty scenes use base dwell, interludes 0.6 times base dwell. Choose `LSF_DWELL=18` for pretty scenes.
The launcher is fullscreen unless you pass `--windowed` or `--size 640x480` for workstation inspection.

## aarch64 build and deployment

`.github/workflows/lsf-build.yml` builds and tests this branch on x86-64 and a native aarch64 Ubuntu 22.04
runner with Rust 1.95.0 and Cargo.lock. It uploads artifacts without publishing a release. This avoids
Fedora-to-Pi glibc cross-linking errors; the target needs glibc >= 2.35 (64-bit Raspberry Pi OS Bookworm or newer).
Push to your own GitHub fork/branch to run it, or dispatch it manually. The workflow has not been executed
from this local workspace. Record the artifact's Git commit and verify the tarball checksum before deploying.

Example from the checkout, after downloading the ARM artifact and extracting its tarball to `release/`:

```sh
# Use the actual checksum filename from the artifact.
(cd dist && sha256sum -c mctl-*.sha256)
# Substitute your SSH host and account. Keep the existing installation for rollback.
ssh pi@mission-control 'mkdir -p ~/lsf-mctl/target/release ~/lsf-mctl/config ~/lsf-mctl/scripts ~/lsf-mctl/packaging/pi'
rsync -av release/mctl pi@mission-control:lsf-mctl/target/release/mctl
rsync -av config/lsf pi@mission-control:lsf-mctl/config/
rsync -av scripts/lsf-display pi@mission-control:lsf-mctl/scripts/
rsync -av packaging/pi/ pi@mission-control:lsf-mctl/packaging/pi/
```

On the Pi, install the graphics/session dependencies (`cage`, `seatd`, `mesa-vulkan-drivers`), then try the
launcher interactively. A Pi 3 uses software Vulkan; a Pi 4 may use V3D. The code now requests only one color
attachment instead of eight, but this still needs validation on the actual Pi 4 driver.

After an interactive check, install the existing service with the launcher as its executable:

```sh
sudo ~/lsf-mctl/packaging/pi/install-display.sh --user pi \
  --bin /home/pi/lsf-mctl/scripts/lsf-display --args '--fullscreen' --dry-run
sudo ~/lsf-mctl/packaging/pi/install-display.sh --user pi \
  --bin /home/pi/lsf-mctl/scripts/lsf-display --args '--fullscreen' --enable
sudo systemctl start mctl-display
journalctl -u mctl-display -f
```

For a dedicated display, after validating console access, select `multi-user.target` so a desktop compositor
does not contend for the display. The existing unit restarts on failure. No SSH target was supplied here,
so installation, boot validation and hardware profiling have not been performed.

## Pi profiling and acceptance

Run at the actual 4:3 output mode (start at 640x480). Record OS, Mesa version, commit, resolution, effect
settings and scene mix with each result. Use `journalctl` and `~/.local/state/mctl/mctl.log` for selected adapter,
source health and the 30-second FPS reports; sample `pidstat -p PID 1 120` if sysstat is installed. Check
`vcgencmd get_throttled` before and after. Compare effects off, scanlines/curvature, then bloom and persistence
one at a time at 5 and 10 fps. A ceiling is not a promise of achieved FPS.

Acceptance requires an unattended full cycle, fresh/failed-source behavior, readable 4:3 text, reboot into
the display, restart after a crash, and a 24-hour soak with bounded memory/temperature. Hardware measurements,
individual view selection, simulation fixtures and that soak are outstanding; no Pi performance claim is made.

## Workstation validation (2026-11-05)

- `cargo build --release --locked --offline -p mctl`: passed on x86-64, Rust 1.95.0.
- `cargo test --workspace --locked --offline`: 359 passed, 6 intentionally ignored. Tests used isolated XDG
  directories; HTTP cache tests needed localhost socket permission outside the filesystem/network sandbox.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: passed. Two upstream boolean
  validation expressions were simplified without changing their handling of NaN/range errors; core tests
  were rerun after that change.
- `cargo fmt --all --check`, shell syntax and service installer dry-run: passed.
- LSF `mctl check`: all three GP sources and IERS EOP passed against live endpoints.
- Headless LSF dense kiosk: eight 640x480 PNGs rendered using cached live elements and llvmpipe Vulkan;
  PHASMA MASTER visually inspected. EGL probing logged driver errors, but Vulkan rendering succeeded.
- No Raspberry Pi measurements, ARM workflow run, reboot test or 24-hour soak yet.


## Expanded inventory and provisional identification

Added on 2026-11-05 after checking the CelesTrak Transporter-18 GP catalog and the international designators
in the SatNOGS TLEs. `[ID?]` means the satellite-to-object association is provisional, not a simulated orbit.
We fetch the real catalog object's GP record by its official NORAD number. Do not remove this flag until
operator/catalog identification is confirmed. Fresh orbital elements do not prove transmitter health.

| Display | SatNOGS temporary ID | COSPAR in SatNOGS TLE | Official NORAD | CelesTrak name |
|---|---:|---|---:|---|
| SIDLOC PQ-03 [ID?] | 98204 | 2026-227CE | 101092 | TRANSPORTER-18 OBJECT CE |
| SIDLOC PQ-04 [ID?] | 98203 | 2026-227CK | 101097 | TRANSPORTER-18 OBJECT CK |
| SIDLOC PQ-05 [ID?] | 98202 | 2026-227CR | 101103 | TRANSPORTER-18 OBJECT CR |
| SIDLOC PQ-06 [ID?] | 98201 | 2026-227CJ | 101096 | TRANSPORTER-18 OBJECT CJ |
| NIS-PQ | 98225 | 2026-227CF | 101093 | NIS-PQ |
| PICOBUS V2 / OSSIE-D | — | 2026-227BC | 100933 | OSSIE D |

Sources: SatNOGS [PQ-03](https://db.satnogs.org/satellite/KPJP-2422-8913-1094-0450/),
[PQ-04](https://db.satnogs.org/satellite/MFOR-9210-8363-3251-4177/),
[PQ-05](https://db.satnogs.org/satellite/KVOS-4539-2765-7785-6036/),
[PQ-06](https://db.satnogs.org/satellite/UNLT-7458-6697-2171-0383/),
[NIS-PQ](https://db.satnogs.org/satellite/ZKZZ-3777-2014-2860-9930/), and
[CelesTrak Transporter-18 GP](https://celestrak.org/NORAD/elements/gp.php?INTDES=2026-227&FORMAT=JSON).
PICOBUS v2 remains on OSSIE-D as confirmed by the operator; it is not modeled as a separately orbiting object.
SIDLOC ARIANE [OFF] likewise follows ARIANE 6 R/B and remains an inactive hosted payload.

## Spacecraft artwork inputs

The current renderer uses rotating 3D line segments, not textured triangle meshes. Upstream models are
procedural Rust geometry in `crates/mctl-gui/src/art.rs`; no OBJ/STL/glTF importer currently exists.
LSF vehicles currently have orbit/ground-track/pass views but no custom spacecraft artwork.

For an accurate lightweight model, supply a simple OBJ with separate body, panels and antennas (few hundred
edges preferred), or STEP/STL to convert. An OBJ is an input for conversion, not yet a drop-in app asset.
For reconstruction from images, provide front/side/top or two to three oblique views, approximate dimensions,
and the deployed antenna/panel configuration. One photo supports only an approximate schematic: hidden
geometry cannot be recovered reliably. For hosted payloads, supply both payload and host placement, so the
view can distinguish SIDLOC on Ariane 6 from PICOBUS v2 on OSSIE-D. Geometry conversion should preserve
structural edges and remove coplanar mesh diagonals to keep Pi rendering cheap.

Expanded-profile validation: both application unit tests passed; `mctl check` accepted nine GP records
plus IERS EOP (all ten sources OK). No renderer or propagator change was required for the extra vehicles.
