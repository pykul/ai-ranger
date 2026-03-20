#!/usr/bin/env bash
# Shared functions for AI Ranger install and update scripts.
# Sourced by scripts/install/*.sh and scripts/update/*.sh.
#
# Provides: detect_target, download_binary, verify_checksum

set -euo pipefail

# GitHub repository owner and name used to construct release download URLs.
readonly GITHUB_REPO="pykul/ai-ranger"

# Base URL for GitHub Releases latest download.
readonly RELEASES_URL="https://github.com/${GITHUB_REPO}/releases/latest/download"

# Checksums file published with every release.
readonly CHECKSUMS_FILE="checksums.txt"

# Detect the Rust target triple for the current platform.
# Outputs: a string like "x86_64-unknown-linux-gnu" or "aarch64-apple-darwin".
detect_target() {
    local os arch target
    os="$(uname -s)"
    arch="$(uname -m)"

    case "${os}" in
        Linux)
            case "${arch}" in
                x86_64)  target="x86_64-unknown-linux-gnu" ;;
                aarch64) target="aarch64-unknown-linux-gnu" ;;
                *)
                    echo "Error: unsupported Linux architecture: ${arch}" >&2
                    exit 1
                    ;;
            esac
            ;;
        Darwin)
            case "${arch}" in
                x86_64)  target="x86_64-apple-darwin" ;;
                arm64)   target="aarch64-apple-darwin" ;;
                *)
                    echo "Error: unsupported macOS architecture: ${arch}" >&2
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Error: unsupported operating system: ${os}" >&2
            exit 1
            ;;
    esac

    echo "${target}"
}

# Download the latest ai-ranger binary for the given target.
# Arguments:
#   $1 - Rust target triple (from detect_target)
#   $2 - Destination path for the binary (e.g. /usr/local/bin/ai-ranger)
download_binary() {
    local target="$1"
    local dest="$2"
    local archive="ai-ranger-${target}.tar.gz"
    local url="${RELEASES_URL}/${archive}"
    local tmpdir

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "${tmpdir}"' EXIT

    echo "Downloading ${archive}..."
    if ! curl -fsSL "${url}" -o "${tmpdir}/${archive}"; then
        echo "Error: failed to download ${url}" >&2
        echo "Check that a release exists at https://github.com/${GITHUB_REPO}/releases" >&2
        exit 1
    fi

    echo "Downloading ${CHECKSUMS_FILE}..."
    if ! curl -fsSL "${RELEASES_URL}/${CHECKSUMS_FILE}" -o "${tmpdir}/${CHECKSUMS_FILE}"; then
        echo "Error: failed to download checksums" >&2
        exit 1
    fi

    verify_checksum "${tmpdir}" "${archive}"

    echo "Extracting binary..."
    tar xzf "${tmpdir}/${archive}" -C "${tmpdir}"

    local dest_dir
    dest_dir="$(dirname "${dest}")"
    mkdir -p "${dest_dir}"
    mv "${tmpdir}/ai-ranger" "${dest}"
    chmod +x "${dest}"

    # Clean up the trap since we are done with tmpdir.
    rm -rf "${tmpdir}"
    trap - EXIT

    echo "Installed ai-ranger to ${dest}"
}

# Verify SHA256 checksum of a downloaded archive.
#
# NOTE: The checksum file is downloaded from the same GitHub Release as the
# binary. This protects against accidental corruption and CDN-level tampering,
# but not against a compromised release (where both the binary and checksums
# could be replaced). For higher assurance deployments, consider verifying
# GPG signatures or using sigstore/cosign once signing is added to the
# release workflow.
#
# Arguments:
#   $1 - Directory containing the archive and checksums.txt
#   $2 - Archive filename to verify
verify_checksum() {
    local dir="$1"
    local archive="$2"

    echo "Verifying checksum..."
    if command -v sha256sum >/dev/null 2>&1; then
        # Linux
        (cd "${dir}" && sha256sum --check "${CHECKSUMS_FILE}" --ignore-missing --quiet)
    elif command -v shasum >/dev/null 2>&1; then
        # macOS
        local expected actual
        expected="$(grep "${archive}" "${dir}/${CHECKSUMS_FILE}" | awk '{print $1}')"
        actual="$(shasum -a 256 "${dir}/${archive}" | awk '{print $1}')"
        if [ "${expected}" != "${actual}" ]; then
            echo "Error: checksum mismatch for ${archive}" >&2
            echo "  Expected: ${expected}" >&2
            echo "  Actual:   ${actual}" >&2
            exit 1
        fi
    else
        echo "Warning: no sha256sum or shasum found, skipping checksum verification" >&2
    fi

    echo "Checksum verified."
}
