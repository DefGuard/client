import './style.scss';

import { isUndefined } from 'lodash-es';
import { useNavigate } from 'react-router-dom';
import { useBreakpoint } from 'use-breakpoint';

import { LogoContainer } from '../../components/LogoContainer/LogoContainer';
import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { deviceBreakpoints } from '../../shared/constants';
import { ArrowSingle } from '../../shared/defguard-ui/components/icons/ArrowSingle/ArrowSingle';
import {
  ArrowSingleDirection,
  ArrowSingleSize,
} from '../../shared/defguard-ui/components/icons/ArrowSingle/types';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { routes } from '../../shared/routes';
import { useEnrollmentStore } from '../enrollment/hooks/store/useEnrollmentStore';

export const SessionTimeoutPage = () => {
  const adminInfo = useEnrollmentStore((state) => state.adminInfo);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const { LL } = useI18nContext();
  const navigate = useNavigate();

  return (
    <PageContainer id="session-timeout">
      <LogoContainer />
      <Card shaded={breakpoint === 'desktop'}>
        <h2>{LL.pages.sessionTimeout.card.header()}</h2>
        <p>{LL.pages.sessionTimeout.card.message()}</p>
      </Card>
      <div className="controls">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          icon={
            <ArrowSingle
              size={ArrowSingleSize.LARGE}
              direction={ArrowSingleDirection.LEFT}
            />
          }
          text={LL.pages.sessionTimeout.controls.back()}
          onClick={() => navigate(routes.client, { replace: true })}
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.pages.sessionTimeout.controls.contact()}
          disabled={isUndefined(adminInfo?.email)}
          onClick={() => {
            if (adminInfo?.email) {
              window.location.href = `mailto:${adminInfo.email}`;
            }
          }}
        />
      </div>
    </PageContainer>
  );
};
