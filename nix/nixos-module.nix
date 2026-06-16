{mkCraneLib}: {
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption mkPackageOption optional types;

  craneLib = mkCraneLib pkgs;
  defguard-client = pkgs.callPackage ./package.nix {inherit pkgs craneLib;};

  svcCfg = config.services.defguard-client-daemon;
  clientCfg = config.programs.defguard-client;
  cliCfg = config.programs.defguard-cli;
in {
  options.services.defguard-client-daemon = {
    enable = mkEnableOption "Defguard VPN client background service (required by both the desktop client and CLI)";

    package = mkPackageOption pkgs "defguard-client" {
      default = ["defguard-client"];
      extraDescription = "Package that provides the defguard-service binary.";
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = "Log level for defguard-service (--log-level)";
    };

    logDir = mkOption {
      type = types.str;
      default = "/var/log/defguard-service";
      description = "Directory for defguard-service logs (--log-dir)";
    };

    statsPeriod = mkOption {
      type = types.int;
      default = 30;
      description = "Interval in seconds for interface statistics updates (--stats-period)";
    };
  };

  options.programs.defguard-client = {
    enable = mkEnableOption "Defguard VPN desktop client";

    package = mkOption {
      type = types.package;
      default = defguard-client;
      description = "defguard-client package to use";
    };
  };

  options.programs.defguard-cli = {
    enable = mkEnableOption "Defguard VPN CLI client (headless)";

    package = mkOption {
      type = types.package;
      default = defguard-client;
      description = "Package that provides the defguard-cli binary.";
    };
  };

  config = mkIf (svcCfg.enable || clientCfg.enable || cliCfg.enable) {
    environment.systemPackages = []
      ++ optional svcCfg.enable svcCfg.package
      ++ optional clientCfg.enable clientCfg.package
      ++ optional cliCfg.enable cliCfg.package;

    systemd.services.defguard-service = {
      description = "Defguard VPN Service";
      documentation = ["https://docs.defguard.net"];
      wantedBy = ["multi-user.target"];
      wants = ["network-online.target"];
      after = ["network-online.target"];
      serviceConfig = {
        Group = "defguard";
        ExecStart = "${svcCfg.package}/bin/defguard-service --log-level ${svcCfg.logLevel} --log-dir ${svcCfg.logDir} --stats-period ${toString svcCfg.statsPeriod}";
        ExecReload = "kill -HUP $MAINPID";
        KillMode = "process";
        KillSignal = "SIGINT";
        LimitNOFILE = 65536;
        LimitNPROC = "infinity";
        Restart = "on-failure";
        RestartSec = 2;
        TasksMax = "infinity";
        OOMScoreAdjust = -1000;
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectControlGroups = true;
        ProtectKernelModules = true;
        RestrictRealtime = true;
        LockPersonality = true;
      };
    };

    users.groups.defguard = {};
  };
}
