import './style.scss';
import { useState } from 'react';
import { Button } from '../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../shared/components/Button/types';
import { Input } from '../../../../shared/components/Input/Input';
import { Snackbar } from '../../../../shared/providers/snackbar/snackbar';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const DEFAULT_TIMEOUT = 2;

export const PlaygroundSnackbarTest = () => {
  const [timeout, setTimeout] = useState<number>(DEFAULT_TIMEOUT);

  return (
    <PlaygroundCard>
      <div className="playground-snackbar-test">
        <h3>Snackbar</h3>
        <Input
          label="Loading timeout (seconds)"
          type="number"
          value={timeout}
          onChange={(v) => setTimeout((v as number) ?? DEFAULT_TIMEOUT)}
        />
        <div className="actions">
          <Button
            text="Clear"
            variant={ButtonVariant.Critical}
            onClick={() => Snackbar.clear()}
          />
        </div>
        <div className="actions">
          <Button
            text="Default"
            variant={ButtonVariant.Secondary}
            onClick={() => Snackbar.default('Default snackbar message')}
          />
          <Button
            text="Success"
            variant={ButtonVariant.Primary}
            onClick={() => Snackbar.success('Operation completed successfully')}
          />
          <Button
            text="Warning"
            variant={ButtonVariant.Outlined}
            onClick={() => Snackbar.warning('Proceed with caution')}
          />
          <Button
            text="Error"
            variant={ButtonVariant.Critical}
            onClick={() => Snackbar.error('Something went wrong')}
          />
          <Button
            text="Loading → Success"
            variant={ButtonVariant.Secondary}
            onClick={() => {
              const anchor = Snackbar.loading('Loading…', 'playground-loading');
              window.setTimeout(() => {
                anchor.update({ text: 'Done!', variant: 'success' });
              }, timeout * 1000);
            }}
          />
        </div>
      </div>
    </PlaygroundCard>
  );
};
