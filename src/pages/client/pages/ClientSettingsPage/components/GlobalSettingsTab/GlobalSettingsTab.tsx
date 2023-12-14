import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { TrayIconTheme } from '../../../../clientAPI/types';
import { useClientStore } from '../../../../hooks/useClientStore';

export const GlobalSettingsTab = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;
  const settings = useClientStore((state) => state.settings);
  const updateClientSettings = useClientStore((state) => state.updateSettings);

  return (
    <div id="global-settings-tab">
      <section>
        <h2>{localLL.tray.title()}</h2>
        <TrayIconThemeSelect />
      </section>
      <section>
        <h2>{localLL.logging.title()}</h2>
      </section>
      <section>
        <h2>{localLL.theme.title()}</h2>
      </section>
    </div>
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
      options={trayThemeSelectOptions}
      selected={settings.tray_icon_theme}
      label={localLL.tray.label()}
      renderSelected={renderSelectedTrayTheme}
      onChangeSingle={(theme) => mutate({ tray_icon_theme: theme })}
      loading={isPending}
    />
  );
};
