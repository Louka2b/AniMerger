.PHONY: all macos linux windows clean fetch

all: fetch macos linux windows

fetch:
	@mkdir -p build/win build/mac build/linux scripts
	@chmod +x scripts/*.sh 2>/dev/null || true
	./scripts/fetch_deps.sh

macos: fetch
	@chmod +x scripts/*.sh
	./scripts/build_macos_app.sh

linux: fetch
	@chmod +x scripts/*.sh
	./scripts/build_linux_appimage.sh

windows: fetch
	cargo build --release --target x86_64-pc-windows-gnu
	@echo "Windows standalone .exe compiled successfully in target/x86_64-pc-windows-gnu/release/animerger.exe !"

clean:
	cargo clean
	rm -rf AppDir dist/*.AppImage dist/*.app build/