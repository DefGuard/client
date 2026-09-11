import './style.scss';
import { LoaderSpinner } from '../../../../../../../shared/components/LoaderSpinner/LoaderSpinner';

export const ConnectModalPostureCheckLoading = () => {
  return (
    <div className="connect-modal-posture-check-loading">
      <LoaderSpinner variant="primary" size={32} />
      <p>Checking device requirements...</p>
    </div>
  );
};
