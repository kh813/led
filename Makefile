.PHONY: all default local tui cli gui test check install package clean help

SHELL := /bin/bash

# Detect OS and Architecture
UNAME_S := $(shell uname -s 2>/dev/null || echo Windows_NT)
UNAME_M := $(shell uname -m 2>/dev/null || echo x86_64)

ifeq ($(UNAME_S),Darwin)
    OS_TYPE := macos
    EXE_EXT :=
    GUI_TARGET := macos-gui
else ifeq ($(findstring MINGW,$(UNAME_S)),MINGW)
    OS_TYPE := windows
    EXE_EXT := .exe
    GUI_TARGET := generic-gui
else ifeq ($(findstring MSYS,$(UNAME_S)),MSYS)
    OS_TYPE := windows
    EXE_EXT := .exe
    GUI_TARGET := generic-gui
else ifeq ($(UNAME_S),Windows_NT)
    OS_TYPE := windows
    EXE_EXT := .exe
    GUI_TARGET := generic-gui
else
    OS_TYPE := linux
    EXE_EXT :=
    GUI_TARGET := generic-gui
endif

DIST_DIR := dist
LED_TUI_BIN := led$(EXE_EXT)
LED_GUI_BIN := led-gui$(EXE_EXT)

# Default: build both TUI and GUI for the current OS
default: local

local: tui gui
	@echo ""
	@echo "==> Build complete for $(OS_TYPE) ($(UNAME_M)) in $(DIST_DIR)/"

all: local

tui: cli
cli:
	@mkdir -p $(DIST_DIR)
	@echo "==> Building TUI (led)..."
	cargo build --release -p led-tui
	@cp target/release/$(LED_TUI_BIN) $(DIST_DIR)/$(LED_TUI_BIN)
	@echo "Built $(DIST_DIR)/$(LED_TUI_BIN)"

gui: $(GUI_TARGET)

macos-gui:
	@mkdir -p $(DIST_DIR)
	@echo "==> Building macOS GUI (led-gui & led.app)..."
	cargo build --release -p led-gui $(CARGO_FLAGS)
	@rm -rf $(DIST_DIR)/led.app
	@mkdir -p $(DIST_DIR)/led.app/Contents/MacOS
	@mkdir -p $(DIST_DIR)/led.app/Contents/Resources
	@BIN_PATH=$$(find target -name $(LED_GUI_BIN) -type f | grep release | head -n 1); \
	if [ -z "$$BIN_PATH" ]; then BIN_PATH="target/release/$(LED_GUI_BIN)"; fi; \
	cp "$$BIN_PATH" $(DIST_DIR)/led.app/Contents/MacOS/$(LED_GUI_BIN); \
	cp "$$BIN_PATH" $(DIST_DIR)/$(LED_GUI_BIN)
	@chmod +x $(DIST_DIR)/led.app/Contents/MacOS/$(LED_GUI_BIN)
	@if [ -f assets/icons/led.icns ]; then cp assets/icons/led.icns $(DIST_DIR)/led.app/Contents/Resources/led.icns; fi
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
	@echo '    <key>CFBundleDisplayName</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>led</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundlePackageType</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>APPL</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleShortVersionString</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>0.1.0</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleVersion</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>0.1.0</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>CFBundleIconFile</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>led.icns</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>NSHighResolutionCapable</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <true/>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>NSSupportsAutomaticGraphicsSwitching</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <true/>' >> $(DIST_DIR)/led.app/Contents/Info.plist
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
	@echo '                <string>public.plain-text</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '            </array>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '        </dict>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    </array>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <key>LSMinimumSystemVersion</key>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '    <string>10.15.7</string>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '</dict>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@echo '</plist>' >> $(DIST_DIR)/led.app/Contents/Info.plist
	@touch $(DIST_DIR)/led.app
	@echo "Built $(DIST_DIR)/led.app and $(DIST_DIR)/$(LED_GUI_BIN)"

generic-gui:
	@mkdir -p $(DIST_DIR)
	@echo "==> Building GUI (led-gui)..."
	cargo build --release -p led-gui $(CARGO_FLAGS)
	@cp target/release/$(LED_GUI_BIN) $(DIST_DIR)/$(LED_GUI_BIN)
	@echo "Built $(DIST_DIR)/$(LED_GUI_BIN)"

test:
	@echo "==> Running workspace tests..."
	cargo test --workspace

check:
	@echo "==> Checking workspace..."
	cargo check --workspace --all-targets

install: local
	@echo "==> Installing binaries..."
	@INSTALL_DIR=$${HOME}/.local/bin; \
	mkdir -p $$INSTALL_DIR; \
	cp $(DIST_DIR)/$(LED_TUI_BIN) $$INSTALL_DIR/; \
	echo "Installed $(LED_TUI_BIN) to $$INSTALL_DIR/"; \
	if [ -f $(DIST_DIR)/$(LED_GUI_BIN) ]; then \
		cp $(DIST_DIR)/$(LED_GUI_BIN) $$INSTALL_DIR/; \
		echo "Installed $(LED_GUI_BIN) to $$INSTALL_DIR/"; \
	fi; \
	if [ "$(OS_TYPE)" = "macos" ] && [ -d $(DIST_DIR)/led.app ]; then \
		mkdir -p $${HOME}/Applications; \
		rm -rf $${HOME}/Applications/led.app; \
		cp -r $(DIST_DIR)/led.app $${HOME}/Applications/; \
		echo "Installed led.app to $${HOME}/Applications/"; \
	fi

package: local
	@echo "==> Packaging release archives..."
	@cd $(DIST_DIR) && \
	if [ "$(OS_TYPE)" = "macos" ]; then \
		tar -czvf led-$(OS_TYPE)-$(UNAME_M).tar.gz $(LED_TUI_BIN); \
		if [ -d led.app ]; then zip -r led-gui-$(OS_TYPE)-$(UNAME_M).zip led.app; fi; \
	elif [ "$(OS_TYPE)" = "windows" ]; then \
		zip -r led-$(OS_TYPE)-$(UNAME_M).zip $(LED_TUI_BIN) $(LED_GUI_BIN); \
	else \
		tar -czvf led-$(OS_TYPE)-$(UNAME_M).tar.gz $(LED_TUI_BIN) $(LED_GUI_BIN); \
	fi
	@echo "Created package archives in $(DIST_DIR)/"

clean:
	rm -rf $(DIST_DIR)
	cargo clean

help:
	@echo "led build targets:"
	@echo "  make              - Build both TUI (led) and GUI (led-gui) for host OS into $(DIST_DIR)/"
	@echo "  make tui          - Build TUI binary (led) into $(DIST_DIR)/"
	@echo "  make gui          - Build GUI binary (and led.app on macOS) into $(DIST_DIR)/"
	@echo "  make test         - Run workspace unit & integration tests"
	@echo "  make check        - Run cargo check across all crates"
	@echo "  make install      - Install binaries to ~/.local/bin (and ~/Applications on macOS)"
	@echo "  make package      - Create release archive (tar.gz / zip) in $(DIST_DIR)/"
	@echo "  make clean        - Remove $(DIST_DIR)/ and target/ build artifacts"
	@echo "  make help         - Show this help message"
