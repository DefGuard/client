import { compareVersions } from 'compare-versions';

import { useApplicationUpdateStore } from './useApplicationUpdateStore';

export const useNewAppVersionAvailable = () => {
  const newAppVersionAvailable = useApplicationUpdateStore((state) => {
    if (!state.currentVersion || !state.latestVersion) return false;

    return compareVersions(state.latestVersion, state.currentVersion) === 1;
  });

  return {
    newAppVersionAvailable,
  };
};
