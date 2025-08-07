{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.programs.defguard-client;
  defguard-client = pkgs.callPackage ./package.nix {};
in {
  options.services.defguard = {
    enable = mkEnableOption "Defguard VPN client and service";

    package = mkOption {
      type = types.package;
      default = defguard-client;
      description = "defguard-client package to use";
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = "Log level for defguard-service";
    };

    statsPeriod = mkOption {
      type = types.int;
      default = 30;
      description = "Interval in seconds for interface statistics updates";
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [cfg.package];

    systemd.services.defguard-service = {
      description = "Defguard VPN Service";
      wantedBy = ["multi-user.target"];
      wants = ["network-online.target"];
      after = ["network-online.target"];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/defguard-service --log-level ${cfg.logLevel} --stats-period ${toString cfg.statsPeriod}";
        Restart = "on-failure";
        RestartSec = 5;
        User = "defguard";
        Group = "defguard";
        StateDirectory = "defguard";
        LogsDirectory = "defguard";
      };
    };

    users.users.defguard = {
      isSystemUser = true;
      group = "defguard";
    };

    users.groups.defguard = {};
  };
}
