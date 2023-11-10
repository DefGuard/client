import './style.scss';

import { useEffect } from 'react';
import { ReactMarkdown } from 'react-markdown/lib/react-markdown';
import { useNavigate } from 'react-router-dom';
import rehypeSanitize from 'rehype-sanitize';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { routes } from '../../../../shared/routes';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const FinishStep = () => {
  const { LL } = useI18nContext();

  const endContent = useEnrollmentStore((state) => state.endContent);
  const navigate = useNavigate();
  const nextSubject = useEnrollmentStore((state) => state.nextSubject);

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      navigate(routes.client.base, { replace: true });
    });
    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject, navigate]);

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
