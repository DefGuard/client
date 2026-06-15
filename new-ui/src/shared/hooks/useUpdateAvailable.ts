import { useQuery } from '@tanstack/react-query';
import { getVersion } from '@tauri-apps/api/app';

import { getLatestAppVersionQueryOptions } from '../rust-api/query';
import { isVersionGreater } from '../utils/compareVersions';

export const useUpdateAvailable = (): boolean => {
  const { data: latest } = useQuery(getLatestAppVersionQueryOptions);
  const { data: currentVersion } = useQuery({
    queryKey: ['app-version'] as const,
    queryFn: () => getVersion(),
  });

  return (
    latest !== undefined &&
    currentVersion !== undefined &&
    isVersionGreater(latest.version, currentVersion)
  );
};
