APP_NAME := animerger
DIST_DIR := dist
WIN_TARGET := x86_64-pc-windows-gnu

.PHONY: all linux appimage windows clean

all: appimage windows

linux:
	cargo build --release
	@mkdir -p $(DIST_DIR)
	cp target/release/$(APP_NAME) $(DIST_DIR)/$(APP_NAME)

appimage: linux
	chmod +x scripts/build_appimage.sh
	./scripts/build_appimage.sh

windows:
	chmod +x scripts/prepare_win_mkvmerge.sh
	./scripts/prepare_win_mkvmerge.sh
	rustup target add $(WIN_TARGET)
	cargo build --release --target $(WIN_TARGET)
	@mkdir -p $(DIST_DIR)
	cp target/$(WIN_TARGET)/release/$(APP_NAME).exe $(DIST_DIR)/$(APP_NAME).exe

clean:
	cargo clean
	rm -rf build $(DIST_DIR)