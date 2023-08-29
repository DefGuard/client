import './style.scss';

import { LogoContainer } from '../../components/LogoContainer/LogoContainer';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { TokenCard } from './components/TokenCard';

export const TokenPage = () => {
  return (
    <PageContainer id="token-page">
      <LogoContainer />
      <TokenCard />
    </PageContainer>
  );
};
