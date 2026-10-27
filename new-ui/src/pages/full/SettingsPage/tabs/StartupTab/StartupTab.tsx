import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Toggle } from '../../../../../shared/components/Toggle/Toggle';
import { api } from '../../../../../shared/rust-api/api';
import { getAutostartEnabledQueryOptions } from '../../../../../shared/rust-api/query';
import type { AppConfig } from '../../../../../shared/rust-api/types';
import { SettingRow } from '../../components/SettingRow/SettingRow';
import { usePatchAppConfig } from '../../hooks/usePatchAppConfig';

type Props = {
  appConfig: AppConfig;
};

export const StartupTab = ({ appConfig }: Props) => {
  const queryClient = useQueryClient();
  const { patchConfig } = usePatchAppConfig();
  const {
    data: autostartEnabled,
    isError: isAutostartStatusError,
    isPending: isAutostartStatusPending,
  } = useQuery(getAutostartEnabledQueryOptions);

  const { mutate: setAutostart, isPending: isAutostartUpdating } = useMutation({
    mutationFn: api.setAutostartEnabled,
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: getAutostartEnabledQueryOptions.queryKey,
      }),
    onError: (error) => {
      console.error(error);
    },
  });

  return (
    <>
      <SettingRow
        title="Start Defguard automatically when your operating system starts"
        inline
      >
        <Toggle
          active={autostartEnabled ?? false}
          disabled={
            isAutostartStatusPending || isAutostartStatusError || isAutostartUpdating
          }
          onClick={() => setAutostart(!autostartEnabled)}
        />
      </SettingRow>
      <SettingRow title="Start the app minimized to the system tray" inline>
        <Toggle
          active={appConfig.start_minimized}
          onClick={() => patchConfig({ start_minimized: !appConfig.start_minimized })}
        />
      </SettingRow>
      <SettingRow
        title="Open app in full window"
        description="Defguard always opens in the full window instead of the compact tray view, at startup and when opened from the system tray."
        inline
      >
        <Toggle
          active={appConfig.open_in_full_window}
          onClick={() =>
            patchConfig({ open_in_full_window: !appConfig.open_in_full_window })
          }
        />
      </SettingRow>
    </>
  );
};
