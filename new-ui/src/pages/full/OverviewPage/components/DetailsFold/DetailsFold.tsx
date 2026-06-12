import './style.scss';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import type { InstanceInfo } from '../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../shared/types';

interface Props {
  data: InstanceInfo;
}

export const DetailsFold = ({ data }: Props) => {
  return (
    <div className="details-fold">
      <div className="group">
        <p>Device configuration</p>
        <div className="card">
          <div className="row">
            <p>Public key</p>
          </div>
          <Divider spacing={ThemeSpacing.Lg} />
          <div className="row">
            <p>Addresses</p>
          </div>
          <Divider spacing={ThemeSpacing.Lg} />
          <div className="row">
            <p>Listen port</p>
          </div>
        </div>
      </div>
      <div className="group">
        <p>VPN Server Configuration</p>
        <div className="card">
          <div className="row">
            <p>Public key</p>
            <p>{data.pubkey}</p>
          </div>
          <Divider spacing={ThemeSpacing.Lg} />
          <div className="row">
            <p>Allowed IPs</p>
          </div>
          <Divider spacing={ThemeSpacing.Lg} />
          <div className="row">
            <p>DNS servers</p>
          </div>
          <Divider spacing={ThemeSpacing.Lg} />
          <div className="row">
            <p>Persistent keep alive</p>
          </div>
          <Divider spacing={ThemeSpacing.Lg} />
          <div className="row">
            <p>Latest Handshake</p>
          </div>
        </div>
      </div>
    </div>
  );
};
