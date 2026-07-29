# Single source of truth for the Node.js + pnpm versions used by both the
# Nix build (newUiDist in package.nix) and the dev shell (shell.nix), so the
# two can't drift. The attribute names are pinned here; the concrete versions
# come from flake.lock's nixpkgs.
pkgs: {
  nodejs = pkgs.nodejs_24;
  pnpm = pkgs.pnpm_11;
}
