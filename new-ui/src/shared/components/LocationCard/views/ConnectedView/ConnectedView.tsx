import './style.scss';
import { LocationMfaMode, MfaMethod } from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { mfaToText } from '../../../../utils/mfa';
import { Divider } from '../../../Divider/Divider';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { ConnectButton } from '../../components/ConnectButton/ConnectButton';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const ConnectedView = () => {
  const { location, setView } = useLocationCardContext();

  const mfaMethod = location.mfa_method ?? MfaMethod.Totp;

  return (
    <div className="location-view-connected">
      {location.location_mfa_mode !== LocationMfaMode.Disabled && mfaMethod && (
        <>
          <div className="location-mfa-row">
            <div className="mfa-badge">
              <p>MFA</p>
            </div>
            <p className="name">{mfaToText(mfaMethod)}</p>
            <IconButton
              variant={IconButtonVariant.SmallSelected}
              icon="edit"
              onClick={() => {
                setView(LocationCardViews.MfaSettings);
              }}
            />
          </div>
          <Divider spacing={ThemeSpacing.Md} />
        </>
      )}
      <SizedBox height={ThemeSpacing.Xl3} />
      <ConnectButton />
    </div>
  );
};
