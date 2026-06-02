import { ThemeSpacing, type ThemeSpacingValue } from '../../types';
import './style.scss';

interface Props {
  title: string;
  spacing?: ThemeSpacingValue;
}
export const FullPageTitle = ({ title, spacing = ThemeSpacing.Md }: Props) => {
  return (
    <div
      className="full-page-title"
      style={{
        paddingBottom: spacing,
      }}
    >
      <p>{title}</p>
    </div>
  );
};
