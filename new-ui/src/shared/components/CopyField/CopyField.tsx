import './style.scss';
import {
  autoUpdate,
  FloatingPortal,
  offset,
  shift,
  useFloating,
} from '@floating-ui/react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import clsx from 'clsx';
import { type HTMLAttributes, type Ref, useEffect, useState } from 'react';
import { isPresent } from '../../utils/isPresent';
import { Icon } from '../Icon';
import { Tooltip } from '../Tooltip/Tooltip';

type Props = {
  text: string;
  label?: string;
  copyTooltip: string;
  ref?: Ref<HTMLDivElement>;
} & HTMLAttributes<HTMLDivElement>;

export const CopyField = ({
  text,
  label,
  ref,
  className,
  copyTooltip,
  ...props
}: Props) => {
  const [copied, setCopied] = useState(false);

  const { refs, floatingStyles } = useFloating({
    placement: 'top',
    whileElementsMounted: autoUpdate,
    middleware: [
      offset(15),
      shift({
        padding: 4,
      }),
    ],
  });

  useEffect(() => {
    if (copied) {
      const clearCopied = () => {
        setCopied(false);
      };
      const timeout = setTimeout(clearCopied, 1500);
      return () => {
        clearTimeout(timeout);
      };
    }
  }, [copied]);

  return (
    <>
      <div className="copy-field spacer">
        <div className="inner">
          {isPresent(label) && (
            <div className="label-track">
              <p>{label}</p>
            </div>
          )}
          <div className={clsx('track', className)} {...props} ref={ref}>
            <p>{text}</p>
            <button
              type="button"
              ref={refs.setReference}
              onClick={() => {
                writeText(text).then(() => {
                  setCopied(true);
                });
              }}
            >
              <Icon icon={!copied ? 'copy' : 'check-filled'} size={20} />
            </button>
          </div>
        </div>
      </div>
      {copied && (
        <FloatingPortal>
          <Tooltip style={floatingStyles} ref={refs.setFloating}>
            <p>{copyTooltip}</p>
          </Tooltip>
        </FloatingPortal>
      )}
    </>
  );
};
