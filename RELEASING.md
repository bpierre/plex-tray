# Releases

The upstream repository is `https://github.com/bpierre/plex-tray`.
The current support target is Arch Linux.

## Check the release

- Run the development checks in README.md and the desktop checks in
  CONTRIBUTING.md.
- Set the version in Cargo.toml and Cargo.lock. Match PKGBUILD's pkgver and
  reset pkgrel when the application version changes.
- Update CHANGELOG.md with the changes included in the release.
- Build with `makepkg --cleanbuild --force` and inspect the package contents
  with `bsdtar -tf`.
- Confirm the package includes its MIT license, icons, desktop entry, and
  Polkit rule.
- Confirm the working tree is clean and generated files are ignored.

## Tag and archive

Create and push a version tag only after the release commit is ready. For
example, after creating `v0.1.0`, archive that exact tag:

```sh
mkdir -p dist
git archive --format=tar --prefix=plex-tray-0.1.0/ v0.1.0 |
  gzip -n > dist/plex-tray-0.1.0.tar.gz
sha256sum dist/plex-tray-0.1.0.tar.gz
```

The archive can be attached to the matching GitHub release. Build binary
packages on the intended target system and keep them out of Git history.
