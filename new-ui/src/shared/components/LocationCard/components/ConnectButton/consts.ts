import { deviceBlockedMessage } from '../../../../../pages/full/DeviceVerificationPage/utils/deviceVerificationErrorMessage';

const connectBlockedTooltip =
  'You can’t connect due to an authentication issue. Contact your administrator.';

export const connectButtonTooltip = ({
  isDeviceBlocked,
  isBlocked,
}: {
  isDeviceBlocked: boolean;
  isBlocked: boolean;
}): string | null => {
  if (isDeviceBlocked) return deviceBlockedMessage;
  if (isBlocked) return connectBlockedTooltip;
  return null;
};
