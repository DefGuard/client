import './style.scss';
import { ThemeSpacing } from '../../../../types';
import { isPresent } from '../../../../utils/isPresent';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Divider } from '../../../Divider/Divider';
import { Icon, IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { DEFAULT_CONNECTION_ERROR } from '../../api/connectError';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const LocationCardErrorView = () => {
  const { setView, cardError, location } = useLocationCardContext();
  const onRetry = cardError?.onRetry;

  const leave = () => {
    setView(location.active ? LocationCardViews.Connected : LocationCardViews.Default);
  };

  return (
    <div className="location-card-error-view">
      <Divider spacing={ThemeSpacing.Md} />
      <SizedBox height={ThemeSpacing.Xl} />
      <Icon icon={cardError?.icon ?? IconKind.ServiceUnavailable} size={24} />
      <SizedBox height={ThemeSpacing.Xl} />
      <p className="description">{cardError?.message ?? DEFAULT_CONNECTION_ERROR}</p>
      <SizedBox height={ThemeSpacing.Xl4} />
      {isPresent(onRetry) && (
        <div className="retry">
          <IconButton
            variant={IconButtonVariant.BigSelected}
            icon={IconKind.ArrowBig}
            iconRotation="left"
            onClick={leave}
          />
          <Button text="Retry" variant={ButtonVariant.Primary} onClick={onRetry} />
        </div>
      )}
      {!isPresent(onRetry) && (
        <Button text="Got it" variant={ButtonVariant.Secondary} onClick={leave} />
      )}
    </div>
  );
};
