import './style.scss';
import clsx from 'clsx';
import type { ReactNode } from 'react';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';

export type SettingRowProps = {
  title: string;
  description?: string;
  children: ReactNode;
  inline?: boolean;
  divider?: boolean;
};

export const SettingRow = ({
  title,
  description,
  children,
  inline = false,
  divider = true,
}: SettingRowProps) => {
  return (
    <div className={clsx('setting-row', { inline })}>
      <div className="head">
        {inline && <div className="control">{children}</div>}
        <p className="title">{title}</p>
      </div>
      {isPresent(description) && (
        <>
          <SizedBox height={ThemeSpacing.Xs} />
          <p className="description">{description}</p>
        </>
      )}
      {!inline && (
        <>
          <SizedBox height={ThemeSpacing.Lg} />
          <div className="control">{children}</div>
        </>
      )}
      {divider && <Divider spacing={ThemeSpacing.Xl} />}
    </div>
  );
};
