APP_NAME := animerger
DIST_DIR := dist
WIN_TARGET := x86_64-pc-windows-gnu
ALASS_VERSION := 2.0.0
ALASS_LINUX_BIN := build/linux/alass
ALASS_ARCHIVE := build/win/alass-windows64.zip
ALASS_BIN := build/win/alass.exe

.PHONY: all linux appimage prepare-linux-alass windows prepare-win-alass macos clean

all: appimage windows

linux:
	cargo build --release
	@mkdir -p $(DIST_DIR)
	cp target/release/$(APP_NAME) $(DIST_DIR)/$(APP_NAME)

appimage: linux prepare-linux-alass
	chmod +x scripts/build_appimage.sh
	./scripts/build_appimage.sh

prepare-linux-alass:
	@mkdir -p build/linux
	@if [ ! -f "$(ALASS_LINUX_BIN)" ]; then \
		echo "Downloading Alass $(ALASS_VERSION)..."; \
		curl -fL -o "$(ALASS_LINUX_BIN)" \
			"https://github.com/kaegi/alass/releases/download/v$(ALASS_VERSION)/alass-linux64"; \
		chmod +x "$(ALASS_LINUX_BIN)"; \
	fi

prepare-win-alass:
	@mkdir -p build/win
	@if [ ! -f "$(ALASS_BIN)" ]; then \
		echo "Downloading Alass $(ALASS_VERSION)..."; \
		curl -fL -o "$(ALASS_ARCHIVE)" \
			"https://github.com/kaegi/alass/releases/download/v$(ALASS_VERSION)/alass-windows64.zip"; \
		unzip -p "$(ALASS_ARCHIVE)" "alass-windows64/bin/alass-cli.exe" > "$(ALASS_BIN)"; \
		rm -f "$(ALASS_ARCHIVE)"; \
		chmod +x "$(ALASS_BIN)"; \
	fi

windows: prepare-win-alass
	chmod +x scripts/prepare_win_mkvmerge.sh
	./scripts/prepare_win_mkvmerge.sh
	rustup target add $(WIN_TARGET)
	cargo build --release --target $(WIN_TARGET)
	@mkdir -p $(DIST_DIR)
	cp target/$(WIN_TARGET)/release/$(APP_NAME).exe $(DIST_DIR)/$(APP_NAME).exe

macos:
	chmod +x scripts/build_macos_app.sh
	./scripts/build_macos_app.sh

clean:
	cargo clean
	rm -rf build $(DIST_DIR)