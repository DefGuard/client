import './style.scss';
import { mfaMethodIcon } from '../../../../consts';
import { MfaMethod } from '../../../../rust-api/types';
import { Icon } from '../../../Icon';

/** CTAP fails the assertion a few seconds after the key starts blinking,
 *  so the prompt has to be unmissable. */
export const Fido2TouchPrompt = () => (
  <div className="fido2-touch-prompt">
    <Icon icon={mfaMethodIcon[MfaMethod.Fido2]} size={28} />
    <p className="prompt">Touch your security key</p>
    <p className="hint">The key is blinking. Tap it to confirm.</p>
  </div>
);
