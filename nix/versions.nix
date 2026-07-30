# Single source of truth for the Node.js + pnpm versions used by both the
# Nix build (newUiDist in package.nix) and the dev shell (shell.nix), so the
# two can't drift.
#
# Node is derived from new-ui/.nvmrc - the same file nvm/fnm and CI read - so
# non-Nix developers, CI, and Nix all track one version. .nvmrc holds the
# major (e.g. "26"); Nix maps it to the matching nixpkgs attribute. The
# concrete patch version still comes from flake.lock's nixpkgs pin.
pkgs: let
  nodeMajor = builtins.head (
    builtins.match "[^0-9]*([0-9]+).*" (builtins.readFile ../new-ui/.nvmrc)
  );
in {
  nodejs = pkgs."nodejs_${nodeMajor}";
  pnpm = pkgs.pnpm_11;
}
