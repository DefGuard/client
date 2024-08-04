import './style.scss';

import { useMutation, useQuery } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { LabeledCheckbox } from '../../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { ThemeKey } from '../../../../../../shared/defguard-ui/hooks/theme/types';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../clientAPI/clientApi';
import { LogLevel, TrayIconTheme } from '../../../../clientAPI/types';
import { useClientStore } from '../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../query';

const { getAppConfig, protectDatabase } = clientApi;

//FIXME: get user input
const protectionPassword = 'DefguardProtect32';

export const GlobalSettingsTab = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;
  const toaster = useToaster();

  const { data: appConfig } = useQuery({
    queryFn: getAppConfig,
    queryKey: [clientQueryKeys],
    refetchOnWindowFocus: false,
  });

  const { mutate: encryptDatabase, isPending } = useMutation({
    mutationFn: protectDatabase,
    onSuccess: () => {
      toaster.success('Database is now protected.');
    },
    onError: (e) => {
      toaster.error('Encryption failed, check console!');
      console.error(e);
    },
  });

  return (
    <div id="global-settings-tab">
      <section>
        <h2>{localLL.versionUpdate.title()}</h2>
        <CheckForUpdatesOption />
      </section>
      <section>
        <h2>{localLL.tray.title()}</h2>
        <TrayIconThemeSelect />
      </section>
      <section>
        <h2>{localLL.logging.title()}</h2>
        <LoggingLevelSelect />
      </section>
      <section>
        <h2>{localLL.theme.title()}</h2>
        <ThemeSelect />
      </section>
      <section>
        <h2>Application database protection</h2>
        <p>status: {String(appConfig?.db_protected)}</p>
        {appConfig && !appConfig.db_protected && (
          <Button
            text="Encrypt Database"
            styleVariant={ButtonStyleVariant.PRIMARY}
            onClick={() => encryptDatabase(protectionPassword)}
            loading={isPending}
          />
        )}
      </section>
    </div>
  );
};

const ThemeSelect = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.theme;
  const settings = useClientStore((state) => state.settings);
  const updateClientSettings = useClientStore((state) => state.updateSettings);
  const { mutate, isPending } = useMutation({
    mutationFn: updateClientSettings,
  });

  const options = useMemo((): SelectOption<ThemeKey>[] => {
    const res: SelectOption<ThemeKey>[] = [
      {
        key: 0,
        label: localLL.options.light(),
        value: 'light',
      },
      {
        key: 1,
        label: localLL.options.dark(),
        value: 'dark',
      },
    ];
    return res;
  }, [localLL.options]);

  const renderSelected = useCallback(
    (theme: ThemeKey): SelectSelectedValue => {
      const option = options.find((o) => o.value === theme);
      if (option) {
        return {
          key: option.key,
          displayValue: option.label,
        };
      }
      return {
        key: 999,
        displayValue: '',
      };
    },
    [options],
  );

  return (
    <Select
      options={options}
      renderSelected={renderSelected}
      selected={settings.theme}
      onChangeSingle={(theme) => mutate({ theme })}
      loading={isPending}
    />
  );
};

const LoggingLevelSelect = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.logging;
  const settings = useClientStore((state) => state.settings);
  const updateClientSettings = useClientStore((state) => state.updateSettings);

  const { mutate, isPending } = useMutation({
    mutationFn: updateClientSettings,
  });

  const loggingOptions = useMemo((): SelectOption<LogLevel>[] => {
    const res: SelectOption<LogLevel>[] = [
      {
        key: 0,
        label: localLL.options.error(),
        value: 'error',
      },
      {
        key: 1,
        label: localLL.options.info(),
        value: 'info',
      },
      {
        key: 2,
        label: localLL.options.debug(),
        value: 'debug',
      },
      {
        key: 3,
        label: localLL.options.trace(),
        value: 'trace',
      },
    ];
    return res;
  }, [localLL.options]);

  const renderSelected = useCallback(
    (val: LogLevel) => {
      const option = loggingOptions.find((o) => o.value === val);
      if (option) {
        return {
          key: option.key,
          displayValue: option.label,
        };
      }
      return {
        key: 999,
        displayValue: '',
      };
    },
    [loggingOptions],
  );

  return (
    <Select
      sizeVariant={SelectSizeVariant.STANDARD}
      options={loggingOptions}
      renderSelected={renderSelected}
      selected={settings.log_level}
      loading={isPending}
      onChangeSingle={(level) => mutate({ log_level: level })}
    />
  );
};

const TrayIconThemeSelect = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;
  const settings = useClientStore((state) => state.settings);
  const updateClientSettings = useClientStore((state) => state.updateSettings);

  const { mutate, isPending } = useMutation({
    mutationFn: updateClientSettings,
  });

  const trayThemeSelectOptions = useMemo((): SelectOption<TrayIconTheme>[] => {
    const res: SelectOption<TrayIconTheme>[] = [
      {
        label: localLL.tray.options.color(),
        value: 'color',
        key: 0,
      },
      {
        value: 'gray',
        key: 1,
        label: localLL.tray.options.gray(),
      },
      {
        value: 'white',
        key: 2,
        label: localLL.tray.options.white(),
      },
      {
        value: 'black',
        key: 3,
        label: localLL.tray.options.black(),
      },
    ];
    return res;
  }, [localLL.tray.options]);

  const renderSelectedTrayTheme = useCallback(
    (theme: TrayIconTheme): SelectSelectedValue => {
      const option = trayThemeSelectOptions.find((t) => t.value === theme);
      if (option) {
        return {
          key: option.key,
          displayValue: option.label,
        };
      }
      return {
        key: 'color',
        displayValue: 'color',
      };
    },
    [trayThemeSelectOptions],
  );

  return (
    <Select
      sizeVariant={SelectSizeVariant.STANDARD}
      options={trayThemeSelectOptions}
      selected={settings.tray_icon_theme}
      label={localLL.tray.label()}
      renderSelected={renderSelectedTrayTheme}
      onChangeSingle={(theme) => mutate({ tray_icon_theme: theme })}
      loading={isPending}
    />
  );
};

const CheckForUpdatesOption = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;
  const settings = useClientStore((state) => state.settings);
  const updateClientSettings = useClientStore((state) => state.updateSettings);
  const { mutate, isPending } = useMutation({
    mutationFn: updateClientSettings,
  });

  return (
    <LabeledCheckbox
      label={localLL.versionUpdate.checkboxTitle()}
      disabled={isPending}
      value={settings.check_for_updates}
      onChange={(value) => {
        mutate({ check_for_updates: value });
      }}
    />
  );
};
