# Defguard Chocolatey package

This directory contains the Chocolatey package source for Defguard.
The GitHub Actions workflow updates version, MSI URL, and checksum after a release is published.

## Workflow behavior

- Trigger: GitHub release `published` (non-prerelease).
- Source MSI: Release asset named `Defguard_<version>_x64_en-US.msi`.
- Updated files:
  - `defguard.nuspec` (`<version>`, `<packageSourceUrl>`)
  - `tools/chocolateyinstall.ps1` (`$url`, `checksum`)
- Package build: `choco pack`.
- Package push: `choco push` to `https://push.chocolatey.org/`.

## Required secret

- `CHOCO_API_KEY` in GitHub repo secrets.

## Local testing (Windows)

From this directory:

```
choco pack
choco install defguard --source .
```
