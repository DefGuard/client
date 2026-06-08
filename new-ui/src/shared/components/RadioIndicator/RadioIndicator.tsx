import './style.scss';
import { useMemo } from 'react';

type Props = {
  hover?: boolean;
  active?: boolean;
  disabled?: boolean;
};

export const RadioIndicator = ({ active, disabled, hover }: Props) => {
  const RenderIcon = useMemo(() => {
    if (active) {
      if (disabled) {
        return StateSelectedDisabled;
      }
      return StateSelected;
    }
    if (disabled) {
      return StateDefaultDisabled;
    }
    if (hover) {
      return StateDefaultHover;
    }
    return StateDefault;
  }, [active, disabled, hover]);

  return (
    <div className="radio-indicator">
      <RenderIcon />
    </div>
  );
};

const StateDefault = () => {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle cx="12" cy="12" r="7.5" stroke="white" />
    </svg>
  );
};

const StateDefaultHover = () => {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle cx="12" cy="12" r="7.5" stroke="white" />
    </svg>
  );
};

const StateDefaultDisabled = () => {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle cx="12" cy="12" r="8" fill="white" fill-opacity="0.05" />
      <circle cx="12" cy="12" r="7.5" stroke="white" stroke-opacity="0.2" />
    </svg>
  );
};

const StateSelected = () => {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle cx="12" cy="12" r="8" fill="white" />
      <circle cx="12" cy="12" r="4" fill="#3961DB" />
    </svg>
  );
};

const StateSelectedDisabled = () => {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle cx="12" cy="12" r="8" fill="white" fill-opacity="0.1" />
      <circle cx="12" cy="12" r="4" fill="white" fill-opacity="0.6" />
    </svg>
  );
};
