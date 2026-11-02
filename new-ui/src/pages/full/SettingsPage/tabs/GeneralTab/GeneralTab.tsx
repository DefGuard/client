import { useMemo } from 'react';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { Input } from '../../../../../shared/components/Input/Input';
import { Select } from '../../../../../shared/components/Select/Select';
import type { SelectOption } from '../../../../../shared/components/Select/types';
import { Toggle } from '../../../../../shared/components/Toggle/Toggle';
import { type AppConfig, LogLevel } from '../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../shared/types';
import { SettingRow } from '../../components/SettingRow/SettingRow';
import { SettingsFooter } from '../../components/SettingsFooter/SettingsFooter';
import { usePatchAppConfig } from '../../hooks/usePatchAppConfig';

const logLevelOptions: SelectOption<LogLevel>[] = [
  { key: LogLevel.Off, label: 'Off', value: LogLevel.Off },
  { key: LogLevel.Error, label: 'Error', value: LogLevel.Error },
  { key: LogLevel.Warn, label: 'Warn', value: LogLevel.Warn },
  { key: LogLevel.Info, label: 'Info', value: LogLevel.Info },
  { key: LogLevel.Debug, label: 'Debug', value: LogLevel.Debug },
  { key: LogLevel.Trace, label: 'Trace', value: LogLevel.Trace },
];

type Props = {
  appConfig: AppConfig;
};

export const GeneralTab = ({ appConfig }: Props) => {
  const { patchConfig } = usePatchAppConfig();

  const selectedLogLevel = useMemo(
    () =>
      logLevelOptions.find((o) => o.value === appConfig.log_level) ?? logLevelOptions[3],
    [appConfig.log_level],
  );

  return (
    <>
      <SettingRow
        title="Logging threshold"
        description="Defines the minimum severity level of events that will be recorded in the logs. Only events at or above this level are captured to reduce noise and improve log clarity."
      >
        <Select
          options={logLevelOptions}
          value={selectedLogLevel}
          onChange={(option) => patchConfig({ log_level: option.value })}
        />
      </SettingRow>

      <SettingRow
        title="Session timeout"
        description="If active connection exceeds given time without making an handshake with the server. The connection will be considered invalid and disconnected automatically."
      >
        <Input
          type="number"
          value={appConfig.peer_alive_period}
          onChange={(value) => {
            if (value === null || value === '') return;
            patchConfig({ peer_alive_period: Number(value) });
          }}
        />
      </SettingRow>

      <SettingRow
        title="Maximum Transmission Unit"
        description="MTU sets the largest packet size sent through the network. Lowering it can improve connection stability in restrictive or unreliable ISP networks. The default value on most systems is 1500. Try lowering it to 1300-1400 if you encounter ISP-related issues."
      >
        <Input
          type="number"
          value={appConfig.mtu}
          onChange={(value) => {
            if (value === null || value === '') return;
            patchConfig({ mtu: Number(value) });
          }}
        />
      </SettingRow>

      <Toggle
        label="Check for updates automatically"
        active={appConfig.check_for_updates}
        onClick={() => patchConfig({ check_for_updates: !appConfig.check_for_updates })}
      />
      <Divider spacing={ThemeSpacing.Xl} />
      <Toggle
        label="Auto start OpenID MFA"
        helper="When connecting to a location protected by OpenID MFA, Defguard skips the sign-in prompt and opens your identity provider's login page in the browser right away."
        active={appConfig.auto_start_openid_mfa}
        onClick={() =>
          patchConfig({ auto_start_openid_mfa: !appConfig.auto_start_openid_mfa })
        }
      />
      <Divider spacing={ThemeSpacing.Xl} />

      <SettingsFooter />
    </>
  );
};
