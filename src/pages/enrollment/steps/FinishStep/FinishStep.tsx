import './style.scss';

import { ReactMarkdown } from 'react-markdown/lib/react-markdown';
import rehypeSanitize from 'rehype-sanitize';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const FinishStep = () => {
  const { LL } = useI18nContext();

  const endContent = useEnrollmentStore((state) => state.endContent);

  return (
    <Card id="enrollment-finish-card">
      <EnrollmentStepIndicator />
      <h3>{LL.pages.enrollment.steps.finish.title()}</h3>
      <div className="content">
        {endContent && endContent.length > 0 && (
          <ReactMarkdown rehypePlugins={[rehypeSanitize]}>{endContent}</ReactMarkdown>
        )}
      </div>
    </Card>
  );
};
