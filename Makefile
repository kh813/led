.PHONY: all default gui clean help

SHELL := /bin/bash

OS := $(shell uname -s)
ARCH := $(shell uname -m)

DIST_DIR := dist
LED_TUI_BIN := led
LED_GUI_BIN := led-gui

default:
	@mkdir -p $(DIST_DIR)
	cargo build --release -p led-tui
	cp target/release/$(LED_TUI_BIN) $(DIST_DIR)/$(LED_TUI_BIN)
	@echo "Built $(DIST_DIR)/$(LED_TUI_BIN)"

all:
	@if [ "$(OS)" != "Darwin" ]; then echo "Error: 'make all' requires macOS host"; exit 1; fi
	@if ! docker info > /dev/null 2>&1; then echo "Error: Docker must be running for Linux cross-compilation"; exit 1; fi
	@mkdir -p $(DIST_DIR)
	# macOS arm64 TUI
	cargo build --release -p led-tui --target aarch64-apple-darwin
	cp target/aarch64-apple-darwin/release/$(LED_TUI_BIN) $(DIST_DIR)/$(LED_TUI_BIN).mac-arm64
	# macOS x64 TUI
	cargo build --release -p led-tui --target x86_64-apple-darwin
	cp target/x86_64-apple-darwin/release/$(LED_TUI_BIN) $(DIST_DIR)/$(LED_TUI_BIN).mac-x64
	# macOS arm64 GUI (stub)
	cargo build --release -p led-gui --target aarch64-apple-darwin
	# .app bundle creation would go here in later phases
	@echo "Built macOS binaries in $(DIST_DIR)/"
	# Linux x64 TUI
	cross build --release -p led-tui --target x86_64-unknown-linux-gnu
	cp target/x86_64-unknown-linux-gnu/release/$(LED_TUI_BIN) $(DIST_DIR)/$(LED_TUI_BIN).linux-x64
	# Linux arm64 TUI
	cross build --release -p led-tui --target aarch64-unknown-linux-gnu
	cp target/aarch64-unknown-linux-gnu/release/$(LED_TUI_BIN) $(DIST_DIR)/$(LED_TUI_BIN).linux-arm64
	@echo "Built Linux binaries in $(DIST_DIR)/"
	@echo "Windows (led.exe): built on GitHub Actions Windows runner (windows-msvc) — not included in make all"

gui:
	@echo "PATH: $$PATH"
	@if [ "$(OS)" != "Darwin" ]; then echo "Error: 'make gui' requires macOS host"; exit 1; fi
	@mkdir -p $(DIST_DIR)
	cargo build --release -p led-gui $(CARGO_FLAGS)
	@rm -rf $(DIST_DIR)/led.app
	@mkdir -p $(DIST_DIR)/led.app/Contents/MacOS
	@mkdir -p $(DIST_DIR)/led.app/Contents/Resources
	@BIN_PATH=$$(find target -name $(LED_GUI_BIN) -type f | grep release | head -n 1); \
	if [ -z "$$BIN_PATH" ]; then BIN_PATH="target/release/$(LED_GUI_BIN)"; fi; \
	cp "$$BIN_PATH" $(DIST_DIR)/led.app/Contents/MacOS/$(LED_GUI_BIN)
	@cp assets/icons/led.icns $(DIST_DIR)/led.app/Contents/Resources/led.icns
	@echo '<?xml version="1.0" encoding="UTF-8"?>' > $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '<plist version="1.0">' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '<dict>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleExecutable</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>$(LED_GUI_BIN)</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleIdentifier</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>dev.hiroshi.led-gui</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleName</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>led</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundlePackageType</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>APPL</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleShortVersionString</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>0.1.0</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleIconFile</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>led.icns</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleDocumentTypes</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <array>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '        <dict>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            <key>CFBundleTypeName</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            <string>All Files</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            <key>CFBundleTypeRole</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            <string>Editor</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            <key>LSItemContentTypes</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            <array>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '                <string>public.data</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '                <string>public.content</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            </array>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '        </dict>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    </array>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>LSMinimumSystemVersion</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>10.15.7</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '</dict>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '</plist>' >> $(DIST_DIR)/led.app/Contents/Info.plist

	@echo "Built $(DIST_DIR)/led.app"

clean:
	rm -rf $(DIST_DIR)
	cargo clean

help:
	@echo "Available targets:"
	@echo "  make        - Build 'led' (TUI) for current host into $(DIST_DIR)/"
	@echo "  make all    - Cross-build all targets (macOS only; Docker required for Linux)"
	@echo "  make gui    - Build 'led.app' (GUI) for current macOS host (stub)"
	@echo "  make clean  - Remove $(DIST_DIR)/ and clean cargo"
	@echo "  make help   - Show this help"
