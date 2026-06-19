import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useNavigate, useSearch } from '@tanstack/react-router';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { DetailsFold } from '../../../shared/components/DetailsFold/DetailsFold';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { getLocationDetailsQueryOptions } from '../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../shared/types';

const fallback = '–';

export const LocationDetailsPage = () => {
  const { locationId, locationName, connectionType } = useSearch({
    from: '/full/_default/location-details',
  });
  const navigate = useNavigate();

  const { data } = useQuery(
    getLocationDetailsQueryOptions({ locationId, connectionType }),
  );

  return (
    <FullPage id="location-details-page">
      <FullPageTitle title={`${locationName} details`} spacing={ThemeSpacing.Xl} />
      <DetailsFold
        sections={[
          {
            title: 'Device configuration',
            rows: [
              { label: 'Public key', value: data?.pubkey ?? fallback },
              { label: 'Addresses', value: data?.address ?? fallback },
              { label: 'Listen port', value: data?.listen_port || fallback },
            ],
          },
          {
            title: 'VPN Server Configuration',
            compact: true,
            rows: [
              { label: 'Public key', value: data?.peer_pubkey ?? fallback },
              { label: 'Allowed IPs:', value: data?.allowed_ips ?? fallback },
              { label: 'DNS servers:', value: data?.dns ?? fallback },
              {
                label: 'Persistent keep alive',
                value: data?.persistent_keepalive_interval ?? fallback,
              },
              {
                label: 'Latest Handshake',
                value:
                  data?.last_handshake != null
                    ? `${data.last_handshake} sec ago`
                    : fallback,
              },
            ],
          },
        ]}
      />
      <div className="page-controls">
        <Button
          text="Back"
          variant={ButtonVariant.Primary}
          onClick={() => navigate({ to: '/full/overview' })}
        />
      </div>
    </FullPage>
  );
};
