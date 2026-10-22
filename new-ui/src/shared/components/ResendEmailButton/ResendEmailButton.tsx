import { Button } from '../Button/Button';
import { ButtonVariant } from '../Button/types';

interface Props {
  sent: boolean;
  onClick: () => void;
  testId?: string;
  loading?: boolean;
  disabled?: boolean;
}

// copy is fixed here so every resend in the app reads the same
export const ResendEmailButton = ({
  sent,
  onClick,
  testId,
  loading = false,
  disabled = false,
}: Props) => {
  return (
    <Button
      text={sent ? 'Email sent' : 'Resend email'}
      variant={ButtonVariant.Secondary}
      testId={testId}
      loading={loading}
      disabled={disabled || sent}
      onClick={() => {
        onClick();
      }}
    />
  );
};
