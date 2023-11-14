#!/bin/bash

SERVICE_BINARY=defguard-service
DAEMON_PROPERTY_FILE=net.defguard.plist
WIREGUARD_GO_BINARY=wireguard-go
DAEMON_NAME=net.defguard
PACKAGE_ID=net.defguard

#Check running user
if (( $EUID != 0 )); then
    echo "Please run as root."
    exit
fi

# Remove wireguard-go shortcut at /usr/local/bin
rm -f /usr/local/bin/${WIREGUARD_GO_BINARY}

# Remove service shortcut at /usr/local/bin
rm -f /usr/local/bin/${SERVICE_BINARY}
ln -s ${PRODUCT_HOME}/${SERVICE_BINARY} /usr/local/bin/${SERVICE_BINARY}

# Remove daemon
launchctl stop ${DAEMON_NAME}
launchctl unload /Library/LaunchDaemons/${DAEMON_PROPERTY_FILE}
rm -f /Library/LaunchDaemons/${DAEMON_PROPERTY_FILE}

pkgutil --forget ${PACKAGE_ID} > /dev/null 2>&1

rm -rf /Applications/defguard.app

echo "Application uninstall process finished"
exit 0
