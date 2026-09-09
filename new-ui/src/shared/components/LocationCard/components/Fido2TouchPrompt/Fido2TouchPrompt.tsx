import './style.scss';
import { Icon } from '../../../Icon';
import { LoaderSpinner } from '../../../LoaderSpinner/LoaderSpinner';

/**
 * Shown while the security key waits for user presence. CTAP gives the user a
 * few seconds to touch the key and then fails the assertion, and the key itself
 * only blinks - so the prompt has to be unmissable.
 */
export const Fido2TouchPrompt = () => (
  <div className="fido2-touch-prompt">
    <div className="key-mark">
      <Icon icon="yubi-keys" size={32} />
      <LoaderSpinner variant="primary" size={56} />
    </div>
    <p className="prompt">Touch your security key</p>
    <p className="hint">The key is blinking. Tap it to confirm.</p>
  </div>
);
