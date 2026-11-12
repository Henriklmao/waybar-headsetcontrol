# Shared helpers for the AUR PKGBUILD generators.
# This file is sourced by generate-pkgbuild*.bash; it is not run directly.

# Canonical description shared by all generated packages.
aur_description='Waybar integration for HeadsetControl with an interactive TUI'

# Resolve the source repository. GitHub Actions sets GITHUB_REPOSITORY to the
# repository being built; local runs fall back to the project URL.
aur_repo_url() {
    printf 'https://github.com/%s' "${GITHUB_REPOSITORY:-Henriklmao/headset-tui}"
}

# Validate a release tag and turn it into an Arch pkgver component.
aur_tag_pkgver() {
    local tag="${1:-}"
    if [[ -z "$tag" ]]; then
        echo "usage: $0 <tag>" >&2
        return 1
    fi
    # The tag is embedded in the generated PKGBUILD, which downstream users
    # build. Restrict it to a version-like token so quotes, '$' or backticks
    # cannot break out of the PKGBUILD strings.
    if [[ ! "$tag" =~ ^v?[0-9]+(\.[0-9]+)*(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
        echo "refusing to generate a PKGBUILD for unexpected tag '$tag'" >&2
        return 1
    fi
    local version="${tag#v}"
    # pkgver must not contain a hyphen (it separates pkgver from pkgrel).
    printf '%s' "${version//-/_}"
}

# Read the crate version from Cargo.toml.
aur_cargo_version() {
    local file="${1:-Cargo.toml}"
    local version
    version=$(grep -m1 '^version' "$file" | cut -d'"' -f2)
    if [[ -z "$version" ]]; then
        echo "could not read the version from $file" >&2
        return 1
    fi
    printf '%s' "$version"
}

# <version>.r<commit-count>.g<short-hash>, the standard VCS pkgver shape.
aur_git_pkgver() {
    local version="$1"
    printf '%s.r%s.g%s' "$version" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

# SHA-256 of a local source file, used by the VCS package.
aur_file_sha256() {
    sha256sum "$1" | cut -d' ' -f1
}
