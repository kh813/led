led icon export
================

Files:
  led_icon.svg          Source file (1024x1024, scalable)
  led.ico               Windows multi-size icon (16/24/32/48/64/128/256px)
  led.iconset/          macOS iconset folder (run iconutil to convert)
  icon_*.png            Individual PNG sizes

How to create macOS .icns:
  On macOS, run:
    iconutil -c icns led.iconset -o led.icns
  This produces led.icns for use in Xcode / led.app/Contents/Resources/

How to use led.ico on Windows:
  Place led.ico in the project root.
  In Cargo.toml (led-gui):
    [package.metadata.winres]
    ICON = "led.ico"
  Or use the `winres` crate in build.rs.

Sizes included in led.ico:
  16x16, 24x24, 32x32, 48x48, 64x64, 128x128, 256x256

Sizes included in led.iconset:
  16, 32, 64, 128, 256, 512px (+ @2x Retina variants)
