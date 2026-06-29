import clsx from 'clsx';
import { isPresent } from '../../../utils/isPresent';
import { Icon } from '../../Icon';
import { InteractionBox } from '../../InteractionBox/InteractionBox';
import type { MenuHeaderProps } from '../types';

export const MenuHeader = ({ text, testId, onHelp, onClose }: MenuHeaderProps) => {
  return (
    <div
      className={clsx('menu-header', {
        'with-help': isPresent(onHelp),
      })}
      data-testid={testId}
    >
      <p className="group-title">{text}</p>
      {isPresent(onHelp) && (
        <InteractionBox
          interactionSize={26}
          className="menu-header-help"
          onClick={() => {
            onClose?.();
            onHelp();
          }}
        >
          <Icon icon="help" size={20} />
        </InteractionBox>
      )}
    </div>
  );
};
