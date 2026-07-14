import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { RenderMarkdown } from '../../../../../shared/components/RenderMarkdown/RenderMarkdown';
import { api } from '../../../../../shared/rust-api/api';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';
import bannerSrc from './assets/banner.png';

export const FinishStep = () => {
  const navigate = useNavigate();
  const response = useEnrollmentStore((s) => s.startResponse);
  const sessionId = useEnrollmentStore((s) => s.sessionId);
  const markdownContent = response?.final_page_content;
  const showMarkdown = isPresent(markdownContent) && markdownContent.length > 0;

  return (
    <div id="finish-step" className="step-content">
      <div className="content-pad">
        <img src={bannerSrc} loading="eager" width={504} height={150} />
        {!showMarkdown && (
          <div className="top">
            <p>{`New Defguard instance added successfully`}</p>
            <p>{`You can now connect this device, check its status and view statistics.`}</p>
          </div>
        )}
        {showMarkdown && <RenderMarkdown content={markdownContent} />}
      </div>
      <Controls>
        <div className="right">
          <Button
            variant={ButtonVariant.Primary}
            text="Got it"
            onClick={() => {
              // Release the enrollment session now that the wizard is done.
              // Best-effort: enrollment_finish errors if the session is already
              // gone, so swallow failures and leave the wizard regardless.
              if (isPresent(sessionId)) {
                void api.enrollmentFinish(sessionId).catch(() => {});
              }
              navigate({ to: '/full/overview', replace: true });
            }}
          />
        </div>
      </Controls>
    </div>
  );
};
