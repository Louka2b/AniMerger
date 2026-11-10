SHELL := /bin/bash

CARGO ?= cargo
export CARGO_TERM_COLOR := always
WINDOWS_TARGET := x86_64-pc-windows-gnu
WINDOWS_RELEASE_BINARY := target/$(WINDOWS_TARGET)/release/animerger.exe
WINDOWS_BINARY := dist/AniMerger.exe
LINUX_APPIMAGE := dist/AniMerger-x86_64.AppImage

RESET := \033[0m
BOLD := \033[1m
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m

.DEFAULT_GOAL := help
.PHONY: all help macos linux windows clean fetch

all: macos linux windows
	@printf '\n$(GREEN)$(BOLD)All builds completed successfully.$(RESET)\n'

help:
	@printf '\n$(CYAN)$(BOLD)AniMerger build commands$(RESET)\n\n'
	@printf '  $(BOLD)make linux$(RESET)    Build the Linux AppImage\n'
	@printf '  $(BOLD)make macos$(RESET)    Build the macOS application\n'
	@printf '  $(BOLD)make windows$(RESET)  Build the Windows executable\n'
	@printf '  $(BOLD)make all$(RESET)      Build every platform target\n'
	@printf '  $(BOLD)make fetch$(RESET)    Check platform dependencies\n'
	@printf '  $(BOLD)make clean$(RESET)    Remove build artifacts\n\n'

fetch:
	@printf '\n$(CYAN)$(BOLD)==> Checking platform dependencies$(RESET)\n'
	@mkdir -p build/win build/mac build/linux scripts
	@chmod +x scripts/*.sh 2>/dev/null || true
	@./scripts/fetch_deps.sh

macos:
	@printf '\n$(CYAN)$(BOLD)==> Building macOS application$(RESET)\n'
	@./scripts/build_macos_app.sh
	@printf '$(GREEN)    Output: dist/AniMerger.app$(RESET)\n'

linux:
	@printf '\n$(CYAN)$(BOLD)==> Building Linux AppImage$(RESET)\n'
	@./scripts/build_appimage.sh
	@printf '$(GREEN)    Output: $(LINUX_APPIMAGE)$(RESET)\n'

windows:
	@printf '\n$(CYAN)$(BOLD)==> Building Windows executable$(RESET)\n'
	@./scripts/prepare_win_mkvmerge.sh
	@$(CARGO) build --release --target $(WINDOWS_TARGET)
	@mkdir -p dist
	@cp $(WINDOWS_RELEASE_BINARY) $(WINDOWS_BINARY)
	@printf '$(GREEN)    Output: $(WINDOWS_BINARY)$(RESET)\n'

clean:
	@printf '$(YELLOW)Removing build artifacts...$(RESET)\n'
	@$(CARGO) clean
	@rm -rf AppDir dist/*.AppImage dist/*.app dist/*.exe build/