{mkCraneLib}: {
  config,
  lib,
  pkgs,
  ...
}: let
  craneLib = mkCraneLib pkgs;
  defguard-client = pkgs.callPackage ./package.nix {inherit pkgs craneLib;};
  cfg = config.programs.defguard-client;
in {
  options.programs.defguard-client = {
    enable = lib.mkEnableOption "Defguard VPN client and service";

    package = lib.mkOption {
      type = lib.types.package;
      default = defguard-client;
      description = "defguard-client package to use";
    };

    logLevel = lib.mkOption {
      type = lib.types.str;
      default = "info";
      description = "Log level for defguard-service";
    };

    statsPeriod = lib.mkOption {
      type = lib.types.int;
      default = 30;
      description = "Interval in seconds for interface statistics updates";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [cfg.package];

    systemd.services.defguard-service = {
      description = "Defguard VPN Service";
      documentation = ["https://docs.defguard.net"];
      wantedBy = ["multi-user.target"];
      wants = ["network-online.target"];
      after = ["network-online.target"];
      serviceConfig = {
        Group = "defguard";
        ExecStart = "${cfg.package}/bin/defguard-service --log-level ${cfg.logLevel} --stats-period ${toString cfg.statsPeriod}";
        ExecReload = "/bin/kill -HUP $MAINPID";
        KillMode = "process";
        KillSignal = "SIGINT";
        LimitNOFILE = 65536;
        LimitNPROC = "infinity";
        Restart = "on-failure";
        RestartSec = 2;
        TasksMax = "infinity";
        OOMScoreAdjust = -1000;
      };
    };

    users.groups.defguard = {};
  };
}
