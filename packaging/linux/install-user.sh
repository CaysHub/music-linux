#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
bin_dir="${HOME}/.local/bin"
app_dir="${HOME}/.local/share/applications"
icon_theme_dir="${HOME}/.local/share/icons/hicolor"

cargo build --release --manifest-path "${repo_dir}/Cargo.toml"
install -Dm755 "${repo_dir}/target/release/music-linux" "${bin_dir}/music-linux"
for size in 16 22 24 32 48 64 96 128 192 256 512; do
    install -Dm644 \
        "${repo_dir}/assets/icons/hicolor/${size}x${size}/apps/music-linux-player.png" \
        "${icon_theme_dir}/${size}x${size}/apps/music-linux-player.png"
done
mkdir -p "${app_dir}"
desktop-file-install \
    --dir="${app_dir}" \
    --set-key=Exec \
    --set-value="${bin_dir}/music-linux" \
    "${repo_dir}/packaging/linux/music-linux.desktop"
update-desktop-database "${app_dir}"
gtk-update-icon-cache --force --ignore-theme-index "${icon_theme_dir}"

echo "Installed Music Player for the current user."
