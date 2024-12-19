Name:     defguard-client
Version:  %{version}
Release:  1%{?dist}
Summary:  Defguard desktop client

License:  Apache-2.0
URL:      https://defguard.net/
Requires: libappindicator-gtk3 webkit2gtk4.0

%description
Desktop client for managing WireGuard VPN connections

%install
%{__mkdir} -p %{buildroot}/%{_bindir}
%{__mkdir} -p %{buildroot}/%{_sbindir}
%{__mkdir} -p %{buildroot}/%{_prefix}/lib/systemd/system
%{__mkdir} -p %{buildroot}/%{_prefix}/lib/defguard-client/resources/icons
%{__mkdir} -p %{buildroot}/%{_datadir}/applications
%{__mkdir} -p %{buildroot}/%{_datadir}/icons/hicolor/128x128/apps
%{__mkdir} -p %{buildroot}/%{_datadir}/icons/hicolor/256x256@2/apps
%{__mkdir} -p %{buildroot}/%{_datadir}/icons/hicolor/32x32/apps
%{__install} -m 755 src-tauri/target/release/defguard-client %{buildroot}/%{_bindir}/
%{__install} -m 755 src-tauri/target/release/defguard-service %{buildroot}/%{_sbindir}/
%{__install} -m 644 src-tauri/target/release/resources/icons/tray-32x32-black.png %{buildroot}/%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-black.png
%{__install} -m 644 src-tauri/target/release/resources/icons/tray-32x32-color.png %{buildroot}/%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-color.png
%{__install} -m 644 src-tauri/target/release/resources/icons/tray-32x32-gray.png %{buildroot}/%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-gray.png
%{__install} -m 644 src-tauri/target/release/resources/icons/tray-32x32-white.png %{buildroot}/%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-white.png
%{__install} -m 644 resources-linux/defguard-service.service %{buildroot}/%{_prefix}/lib/systemd/system/
%{__install} -m 644 resources-linux/defguard-client.desktop %{_datadir}/applications/defguard-client.desktop
%{__install} -m 644 src-tauri/icons/128x128.png %{_datadir}/icons/hicolor/128x128/apps/defguard-client.png
%{__install} -m 644 src-tauri/icons/128x128@2x.png %{_datadir}/icons/hicolor/256x256@2/apps/defguard-client.png
%{__install} -m 644 src-tauri/icons/32x32.png %{_datadir}/icons/hicolor/32x32/apps/defguard-client.png

%post
# %{systemd_post} defguard-service.service
if [ $1 -eq 1 ]; then
    systemctl daemon-reload
    systemctl enable defguard-service.service
    systemctl start defguard-service.service
fi

%preun
# %{systemd_preun} defguard-service.service
if [ $1 -eq 0 ]; then
    systemctl stop defguard-service.service
    systemctl disable defguard-service.service
fi

%postun
# %{systemd_postun} defguard-service.service
systemctl daemon-reload

%files
%{_bindir}/defguard-client
%{_sbindir}/defguard-service
%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-black.png
%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-color.png
%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-gray.png
%{_prefix}/lib/defguard-client/resources/icons/tray-32x32-white.png
%{_prefix}/lib/systemd/system/defguard-service.service
%{_datadir}/applications/defguard-client.desktop
%{_datadir}/icons/hicolor/128x128/apps/defguard-client.png
%{_datadir}/icons/hicolor/256x256@2/apps/defguard-client.png
%{_datadir}/icons/hicolor/32x32/apps/defguard-client.png
