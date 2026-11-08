#!/bin/sh
# Fabrique gestionnaire-maj_<version>_amd64.deb à la racine du projet.
set -eu
cd "$(dirname "$0")/.."

PKG=gestionnaire-maj
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=$(dpkg --print-architecture)
DEB="${PKG}_${VERSION}_${ARCH}.deb"
ROOT=target/deb/$PKG

cargo build --release
rm -rf "$ROOT"
install -Dm755 target/release/gestionnaire-maj "$ROOT/usr/bin/$PKG"
install -Dm755 target/release/gestionnaire-maj-aide "$ROOT/usr/libexec/$PKG/aide"
install -Dm755 packaging/actualiser "$ROOT/usr/libexec/$PKG/actualiser"
install -Dm755 packaging/auto-maj "$ROOT/usr/libexec/$PKG/auto-maj"
install -Dm644 packaging/org.gestionnairemaj.policy "$ROOT/usr/share/polkit-1/actions/org.gestionnairemaj.policy"
install -Dm644 packaging/gestionnaire-maj.desktop "$ROOT/usr/share/applications/$PKG.desktop"
install -Dm644 packaging/gestionnaire-maj.svg "$ROOT/usr/share/icons/hicolor/scalable/apps/$PKG.svg"
install -Dm644 README.md "$ROOT/usr/share/doc/$PKG/README.md"
cat > "$ROOT/usr/share/doc/$PKG/copyright" <<COPYRIGHT
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: gestionnaire-maj

Files: *
Copyright: 2026 Jean-Baptiste Andrivet
License: GPL-3+
 On Debian systems, the full text of the GNU General Public License
 version 3 can be found in /usr/share/common-licenses/GPL-3.
COPYRIGHT
printf '%s (%s) unstable; urgency=low\n\n  * Version %s.\n\n -- Jean-Baptiste Andrivet <jeanbaptisteandrivet@gmail.com>  %s\n' \
  "$PKG" "$VERSION" "$VERSION" "$(LC_ALL=C date -R)" | gzip -9n > "$ROOT/usr/share/doc/$PKG/changelog.gz"

chmod 644 "$ROOT/usr/share/doc/$PKG/"*
mkdir -p "$ROOT/DEBIAN"
cat > "$ROOT/DEBIAN/control" <<CONTROL
Package: $PKG
Version: $VERSION
Section: admin
Priority: optional
Architecture: $ARCH
Installed-Size: $(du -sk "$ROOT/usr" | cut -f1)
Depends: libc6 (>= 2.34), libgcc-s1, apt, curl, pkexec | policykit-1, polkitd | policykit-1, libgl1, libegl1, libxkbcommon0, libwayland-client0, libwayland-cursor0, libwayland-egl1, libx11-6, libxcursor1, libxrandr2, libxi6
Recommends: ubuntu-drivers-common, pciutils, fonts-dejavu-core
Suggests: snapd, flatpak
Maintainer: Jean-Baptiste Andrivet <jeanbaptisteandrivet@gmail.com>
Description: Gestionnaire de mises à jour et de pilotes au look Ubuntu Human (2008)
 Affiche les mises à jour disponibles (sécurité, recommandées, Snap, Flatpak),
 permet de choisir lesquelles installer et suit l'installation pas à pas.
 L'onglet Pilotes trouve les pilotes recommandés pour le matériel (NVIDIA,
 Wi-Fi…) et les active ou les désactive d'un clic.
 .
 Une case « Lancer au démarrage de l'ordinateur » démarre une veille avec la
 session : quand des mises à jour arrivent, une bulle apparaît avec un bouton
 « Installer ». Le gestionnaire se met lui-même à jour depuis GitHub.
CONTROL

# Première installation (ou depuis une version < 1.3.0) : ne proposer que les versions LTS
cat > "$ROOT/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
    if [ -z "$2" ] || dpkg --compare-versions "$2" lt 1.3.0; then
        f=/etc/update-manager/release-upgrades
        if [ -f "$f" ]; then
            if grep -q '^Prompt=' "$f"; then
                sed -i 's/^Prompt=.*/Prompt=lts/' "$f"
            else
                printf 'Prompt=lts\n' >> "$f"
            fi
        else
            mkdir -p /etc/update-manager
            printf '[DEFAULT]\nPrompt=lts\n' > "$f"
        fi
    fi
fi
exit 0
POSTINST
chmod 755 "$ROOT/DEBIAN/postinst"

# À la désinstallation, arrêter les veilles qui tournent encore
cat > "$ROOT/DEBIAN/prerm" <<'PRERM'
#!/bin/sh
set -e
if [ "$1" = "remove" ]; then
    pkill -f "/usr/bin/gestionnaire-maj --veille" 2>/dev/null || true
fi
exit 0
PRERM
chmod 755 "$ROOT/DEBIAN/prerm"

dpkg-deb --root-owner-group -Zxz -b "$ROOT" "$DEB"
echo "$DEB"
