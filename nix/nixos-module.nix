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
        # Add capabilities to manage network interfaces
        CapabilityBoundingSet = "CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_MODULE";
        AmbientCapabilities = "CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_MODULE";
        # Allow access to /dev/net/tun for TUN/TAP devices
        DeviceAllow = "/dev/net/tun rw";
        # Access to /sys for network configuration
        BindReadOnlyPaths = [
          "/sys"
          "/proc"
        ];
        # Protect the system while giving necessary access
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        # Allow the service to manage network namespaces
        PrivateNetwork = false;
      };
    };

    users.users.defguard = {
      isSystemUser = true;
      group = "defguard";
    };

    users.groups.defguard = {};
  };
}
