import './style.scss';
import { ThemeSpacing } from '../../../../types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Divider } from '../../../Divider/Divider';
import { Icon, IconKind } from '../../../Icon';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { DEFAULT_CONNECTION_ERROR } from '../../api/connectError';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const LocationCardConnectionErrorView = () => {
  const { setView, connectionError } = useLocationCardContext();

  return (
    <div className="location-card-connection-error-view">
      <Divider spacing={ThemeSpacing.Md} />
      <SizedBox height={ThemeSpacing.Xl} />
      <Icon icon={IconKind.ServiceUnavailable} size={24} />
      <SizedBox height={ThemeSpacing.Xl} />
      <p className="description">{connectionError ?? DEFAULT_CONNECTION_ERROR}</p>
      <SizedBox height={ThemeSpacing.Xl} />
      <Button
        text="Back"
        variant={ButtonVariant.Primary}
        onClick={() => setView(LocationCardViews.Default)}
      />
      <SizedBox height={ThemeSpacing.Xl} />
    </div>
  );
};
