Screenshots and GIF Capture (Linux)

Place captured images/GIFs under `assets/screenshots/` using these names so the README picks them up:
- ui-toolbar.png — Top toolbar with buttons and accent picker
- ui-search.png — Search bar with matches visible
- ui-image-view.png — Image viewer with controls visible
- demo.gif — Short demo showing open, search, and image zoom

Options to capture:
- GNOME Screenshot: Press `PrtSc` / `Shift+PrtSc` and save to `assets/screenshots/`
- KDE Spectacle: Launch Spectacle, select region/window, save to `assets/screenshots/`
- Flameshot (X11/Wayland): `flameshot gui` then save to `assets/screenshots/`
- OBS Studio (record GIF via export): Record a short clip and export as GIF

Record a quick GIF with ffmpeg (X11 example):
1) Find your screen size: `xdpyinfo | rg dimensions`
2) Record a region (edit `1920x1080+X+Y`):
   ffmpeg -f x11grab -s 1920x1080 -i :0.0+100,100 -r 30 -t 8 -y /tmp/demo.mp4
3) Convert to GIF (via ffmpeg only):
   ffmpeg -i /tmp/demo.mp4 -vf "fps=15,scale=1024:-1:flags=lanczos" -y assets/screenshots/demo.gif

Wayland tip:
- Use `gtk-recordmydesktop`, `wf-recorder` (wlroots), or OBS. Then convert MP4 → GIF as above.

After saving files with the specified names, `git add assets/screenshots/*` and push.

