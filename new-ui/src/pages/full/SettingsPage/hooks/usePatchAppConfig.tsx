import { useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../../../shared/rust-api/api';
import { getAppConfigQueryOptions } from '../../../../shared/rust-api/query';
import type { AppConfigPatch } from '../../../../shared/rust-api/types';

export const usePatchAppConfig = () => {
  const queryClient = useQueryClient();

  const { mutate: patchConfig } = useMutation({
    mutationFn: (patch: AppConfigPatch) => api.setAppConfig(patch, true),
    onSuccess: (config) => {
      queryClient.setQueryData(getAppConfigQueryOptions.queryKey, config);
    },
    onError: (error) => {
      console.error(error);
    },
  });

  return { patchConfig };
};
