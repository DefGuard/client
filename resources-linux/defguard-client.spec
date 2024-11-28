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
%{__mkdir} -p %{buildroot}/%{_bindir} %{buildroot}/%{_libdir} %{buildroot}/%{_sbindir}
%{__install} -m 755 src-tauri/target/release/defguard-client %{buildroot}/%{_bindir}/
%{__install} -m 755 src-tauri/target/release/defguard-service %{buildroot}/%{_sbindir}/
%{__install} -m 644 resources-linux/defguard-service.service %{buildroot}/%{_libdir}/systemd/system/

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
%{_libdir}/systemd/system/defguard-service.service
