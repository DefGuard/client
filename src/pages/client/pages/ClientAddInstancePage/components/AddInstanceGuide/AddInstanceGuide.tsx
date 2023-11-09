import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';

export const AddInstanceGuide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addInstancePage.guide;
  return (
    <section id="add-instnace-guide">
      <div>
        <h2>{localLL.title()}</h2>
        <p>{localLL.subTitle()}</p>
      </div>
      <Card>
        <h2>{localLL.card.title()}</h2>
        {localLL.card.content()}
      </Card>
    </section>
  );
};
