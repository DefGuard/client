import clsx from 'clsx';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import type { JSX, PropsWithChildren } from 'react';
import { IconButton } from '../../../shared/components/IconButton/IconButton';
import { IconButtonVariant } from '../../../shared/components/IconButton/types';
import { api } from '../../../shared/rust-api/api';

interface Props extends PropsWithChildren {
  containerProps?: JSX.IntrinsicElements['main'];
}

export const CompactPage = ({ children, containerProps }: Props) => {
  const { mutate: closeWindow, isPending } = useMutation({
    mutationFn: api.closeTrayWindow,
  });

  return (
    <main {...containerProps} className={clsx('compact-page', containerProps?.className)}>
      <div className="close-window">
        <IconButton
          variant={IconButtonVariant.SmallSelected}
          icon="close"
          onClick={() => {
            if (!isPending) {
              closeWindow();
            }
          }}
        />
      </div>
      {children}
    </main>
  );
};
