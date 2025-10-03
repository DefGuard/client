#!/bin/bash

SERVICE_BINARY=defguard-service
DAEMON_PROPERTY_FILE=net.defguard.plist
DAEMON_NAME=net.defguard
PACKAGE_ID=net.defguard

#Check running user
if (( $EUID != 0 )); then
    echo "Please run as root."
    exit
fi

# Remove service shortcut at /usr/local/bin
rm -f /usr/local/bin/${SERVICE_BINARY}

# Remove daemon
launchctl stop ${DAEMON_NAME}
launchctl unload /Library/LaunchDaemons/${DAEMON_PROPERTY_FILE}
rm -f /Library/LaunchDaemons/${DAEMON_PROPERTY_FILE}

pkgutil --forget ${PACKAGE_ID} > /dev/null 2>&1

rm -rf /Applications/defguard-client.app

echo "Application uninstall process finished"
