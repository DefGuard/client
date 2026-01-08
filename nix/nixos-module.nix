{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  defguard-client = pkgs.callPackage ./package.nix {};
  cfg = config.programs.defguard-client;
in {
  options.programs.defguard-client = {
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
    # Add client package
    environment.systemPackages = [cfg.package];

    # Setup systemd service for the intrerface management daemon
    systemd.services.defguard-service = {
      description = "Defguard VPN Service";
      wantedBy = ["multi-user.target"];
      wants = ["network-online.target"];
      after = ["network-online.target"];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/defguard-service --log-level ${cfg.logLevel} --stats-period ${toString cfg.statsPeriod}";
        ExecReload = "/bin/kill -HUP $MAINPID";
        Group = "defguard";
        Restart = "on-failure";
        RestartSec = 2;
        KillMode = "process";
        KillSignal = "SIGINT";
        LimitNOFILE = 65536;
        LimitNPROC = "infinity";
        TasksMax = "infinity";
        OOMScoreAdjust = -1000;
      };
    };

    # Make sure the defguard group exists
    users.groups.defguard = {};
  };
}
