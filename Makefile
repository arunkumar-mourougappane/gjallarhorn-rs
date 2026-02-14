PREFIX ?= /usr/local
BIN_DIR = $(PREFIX)/bin
DESKTOP_DIR = $(PREFIX)/share/applications
ICON_DIR = $(PREFIX)/share/icons/hicolor/scalable/apps

build:
	cargo build --release

install: build
	install -d $(BIN_DIR)
	install -m 755 target/release/gjallarhorn $(BIN_DIR)/gjallarhorn
	install -d $(DESKTOP_DIR)
	install -m 644 gjallarhorn.desktop $(DESKTOP_DIR)/gjallarhorn.desktop
	# install -d $(ICON_DIR)
	# install -m 644 icon.svg $(ICON_DIR)/gjallarhorn.svg

uninstall:
	rm -f $(BIN_DIR)/gjallarhorn
	rm -f $(DESKTOP_DIR)/gjallarhorn.desktop

clean:
	cargo clean
