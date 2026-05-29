import './style.scss';
import { ThemeSpacing } from '../../../../types';
import { Divider } from '../../../Divider/Divider';
import { LoaderSpinner } from '../../../LoaderSpinner/LoaderSpinner';

export const LocationCardMfaStartLoader = () => (
  <div className="mfa-start-loader">
    <Divider spacing={ThemeSpacing.Md} />
    <div className="loader-content">
      <LoaderSpinner variant="primary" size={32} />
      <p>Checking device requirements...</p>
    </div>
  </div>
);
