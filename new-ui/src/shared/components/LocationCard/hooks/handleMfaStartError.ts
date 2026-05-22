import type { LocationInfo } from '../../../rust-api/types';
import { shouldShowPostureError } from '../api/startClientMfaSession';
import { LocationCardViews, type LocationCardViewsValue } from '../context/types';

type HandleMfaStartErrorParams = {
  err: unknown;
  location: LocationInfo;
  setPostureError: (error: string | null) => void;
  setView: (view: LocationCardViewsValue) => void;
};

/** Handles MFA start posture failures and reports whether the error was consumed. */
export const handleMfaStartError = ({
  err,
  location,
  setPostureError,
  setView,
}: HandleMfaStartErrorParams): boolean => {
  if (!shouldShowPostureError(err, location)) {
    return false;
  }

  setPostureError(err.message);
  setView(LocationCardViews.PostureCheckFail);
  return true;
};
