{mkCraneLib}: {
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkDefault mkEnableOption mkIf mkMerge mkOption optional types;

  craneLib = mkCraneLib pkgs;
  defguard-client = pkgs.callPackage ./package.nix {inherit pkgs craneLib;};

  svcCfg = config.services.defguard-client-daemon;
  clientCfg = config.programs.defguard-client;
in {
  options.services.defguard-client-daemon = {
    enable = mkEnableOption "Defguard VPN client background service (required by both the desktop client and CLI)";

    package = mkOption {
      type = types.package;
      default = defguard-client;
      description = "Package that provides the defguard-service binary.";
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

  config = mkMerge [
    # Auto-enable the daemon when the desktop client is enabled.
    # Users can override with services.defguard-client-daemon.enable = false.
    {
      services.defguard-client-daemon.enable = mkDefault clientCfg.enable;
    }

    # Add the relevant packages to the system PATH.
    (mkIf (svcCfg.enable || clientCfg.enable) {
      environment.systemPackages =
        []
        ++ optional svcCfg.enable svcCfg.package
        ++ optional clientCfg.enable clientCfg.package;
    })

    # Daemon-only configuration: systemd service and dedicated group.
    (mkIf svcCfg.enable {
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
    })
  ];
}
