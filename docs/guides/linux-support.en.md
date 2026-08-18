# Linux Support Guide

Linux support for Overmax is still in its early stages. It is not as broadly supported as Windows; only Proton/XWayland environments that meet the conditions below are currently supported.

## Supported Environments

- x86_64 Linux with glibc 2.35 or later
- A Wayland compositor that supports `wlr-layer-shell`
- XWayland running in the same session
- Vulkan drivers
- fontconfig and a font with Korean glyphs
- DJMAX RESPECT V running through Proton/XWayland on the same `DISPLAY`
- Borderless fullscreen
- Windowed-mode capture and recognition on a single output
- In windowed mode, the overlay supports manual screen-relative placement only and does not automatically follow the game window

## Checking Your Environment

Run the following commands from the extracted directory:

```bash
uname -m
getconf GNU_LIBC_VERSION
printf 'session=%s WAYLAND_DISPLAY=%s DISPLAY=%s\n' \
  "${XDG_SESSION_TYPE:-unset}" "${WAYLAND_DISPLAY:-unset}" "${DISPLAY:-unset}"
ldd ./overmax
fc-match ':lang=ko' | head -n 1
```

- `uname -m`: `x86_64`
- `getconf GNU_LIBC_VERSION`: `glibc 2.35` or later
- Session: `session=wayland`, with both `WAYLAND_DISPLAY` and `DISPLAY` set
- `ldd`: no shared libraries reported as `not found`
- `fc-match`: a Korean-capable font is shown

## Installation and Launch

1. Download `overmax-linux-x86_64.tar.gz` from Releases.
2. Extract it into a directory where your user has write permission.
3. Run DJMAX RESPECT V through Proton/XWayland in borderless fullscreen or windowed mode.
4. Run `./overmax` from a terminal in the same desktop session.

Settings and caches are stored in the application directory. The directory containing the executable must be writable for automatic updates. When updating manually, copy `settings.user.json` and `cache/` as well.

## Launching Without a Terminal

In Overmax settings, select **System → Launch Linux App → Create Shortcut** to create an `overmax.desktop` entry in your user application menu. It uses the current executable and application directory. You can then launch Overmax from the application menu.

If you move the installation directory, the old shortcut will still point to the previous location. Run Overmax from its new location and create the shortcut again.

## Starting Overmax with the Game from Steam

Enter the following command under DJMAX RESPECT V **Properties → General → Launch Options** in Steam. Replace `OVERMAX_DIR` with the directory where you extracted Overmax.

```bash
sh -c '(cd "OVERMAX_DIR" && exec ./overmax) & exec "$@"' -- %command%
```

## Troubleshooting Startup Problems

Run `./overmax` from a terminal, check the first error shown, and apply the corresponding solution below.

| Symptom or check result | Solution |
| --- | --- |
| `Permission denied` | Run `chmod +x ./overmax`. |
| `Exec format error`, or `uname -m` is not `x86_64` | The current release bundle runs only on x86_64. |
| `GLIBC_2.35 not found`, or glibc is older than 2.35 | Run Overmax on a distribution with glibc 2.35 or later. |
| `ldd` reports `not found` | Install the distribution package that provides the reported shared library. |
| `WAYLAND_DISPLAY is not set` | Log out, sign in to a Wayland session, and launch Overmax from a terminal in that session. |
| `DISPLAY is not set` or `X11 connect failed` | Enable XWayland and run the game and Overmax in the same desktop session. |
| Overmax exits immediately without an error | Run `pgrep -a overmax` to check whether Overmax is already running. If it is not, try again from a normal desktop session with `XDG_RUNTIME_DIR` set. |
| `zwlr_layer_shell_v1 is unavailable` | Use a compositor that supports `wlr-layer-shell`. |
| Vulkan adapter, device, or surface error | Install or update the Vulkan driver and loader provided by your GPU vendor. |
| `Composite` or `MIT-SHM` error | Use a session with the XComposite and MIT-SHM XWayland extensions enabled. Gamescope sessions are not currently supported. |
| Korean text is missing, or `fc-match` fails | Install fontconfig and a Korean-capable font, then run `fc-cache -f`. |
| `DJMAX RESPECT V window not found` | Start the game first. Confirm that Proton uses XWayland rather than native Wayland and that the game and Overmax use the same `DISPLAY`. |
| The game window is found, but the overlay is not displayed correctly | Switch the game to borderless fullscreen. |
| The overlay does not follow the game after moving it in windowed mode | Reposition the overlay manually. |
| Permission error while saving settings or caches | Extract the bundle again into a directory where your user has write permission. |
| Overmax does not start after an update | Extract the entire new bundle and copy only the existing `settings.user.json` and `cache/`. Do not mix it with old executables or shared libraries. |

## Currently Unsupported Features and Environments

- Automatic overlay placement that follows the game window in windowed mode
- Windowed mode across multiple outputs
- Gamescope and Steam Deck Gaming Mode
- Linux system tray icon

## Environments with Limited Validation

The following environments have not been tested enough to guarantee compatibility:

- Differences between compositors and distributions
- GPU vendor and driver combinations
- Exclusive fullscreen
- Fractional scaling and HiDPI combinations
- Different Proton versions
