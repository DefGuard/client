export const Direction = {
  UP: 'up',
  DOWN: 'down',
  LEFT: 'left',
  RIGHT: 'right',
} as const;

export type DirectionValue = (typeof Direction)[keyof typeof Direction];

export const Orientation = {
  Horizontal: 'horizontal',
  Vertical: 'vertical',
} as const;

export type OrientationValue = (typeof Orientation)[keyof typeof Orientation];

export const ThemeSpacing = {
  Xs: 'var(--spacing-xs)',
  Sm: 'var(--spacing-sm)',
  Md: 'var(--spacing-md)',
  Lg: 'var(--spacing-lg)',
  Xl: 'var(--spacing-xl)',
  Xl2: 'var(--spacing-2xl)',
  Xl3: 'var(--spacing-3xl)',
  Xl4: 'var(--spacing-4xl)',
  Xl5: 'var(--spacing-5xl)',
  Xl6: 'var(--spacing-6xl)',
  Xl7: 'var(--spacing-7xl)',
  Xl8: 'var(--spacing-8xl)',
  Xl9: 'var(--spacing-9xl)',
} as const;

export type ThemeSpacingValue = (typeof ThemeSpacing)[keyof typeof ThemeSpacing];

export const ThemeVariable = {
  BgWhite100: 'var(--bg-white-100)',
  BgWhite90: 'var(--bg-white-90)',
  BgWhite80: 'var(--bg-white-80)',
  BgWhite70: 'var(--bg-white-70)',
  BgWhite60: 'var(--bg-white-60)',
  BgWhite50: 'var(--bg-white-50)',
  BgWhite40: 'var(--bg-white-40)',
  BgWhite30: 'var(--bg-white-30)',
  BgWhite20: 'var(--bg-white-20)',
  BgWhite10: 'var(--bg-white-10)',
  BgWhite5: 'var(--bg-white-5)',
  FgWhite100: 'var(--fg-white-100)',
  FgWhite90: 'var(--fg-white-90)',
  FgWhite80: 'var(--fg-white-80)',
  FgWhite70: 'var(--fg-white-70)',
  FgWhite60: 'var(--fg-white-60)',
  FgWhite50: 'var(--fg-white-50)',
  FgWhite40: 'var(--fg-white-40)',
  FgWhite30: 'var(--fg-white-30)',
  FgWhite20: 'var(--fg-white-20)',
  FgWhite10: 'var(--fg-white-10)',
  FgWhite5: 'var(--fg-white-5)',
  BgCritical: 'var(--bg-critical)',
  BgNeutral: 'var(--bg-neutral)',
  BgDarkBlue60: 'var(--bg-dark-blue-60)',
  BgDarkBlue40: 'var(--bg-dark-blue-40)',
  BgDarkBlue30: 'var(--bg-dark-blue-30)',
  BgDarkBlue20: 'var(--bg-dark-blue-20)',
  BgSuccess: 'var(--bg-success)',
  BgWarning: 'var(--bg-warning)',
  BgCriticalFaded: 'var(--bg-critical-faded)',
  BgCriticalMuted: 'var(--bg-critical-muted)',
  BgCriticalDisabled: 'var(--bg-critical-disabled)',
  BorderBg: 'var(--border-bg)',
  BorderAction: 'var(--border-action)',
  BorderActionDisabled: 'var(--border-action-disabled)',
  BorderDefault: 'var(--border-default)',
  BorderDisabled: 'var(--border-disabled)',
  BorderEmphasis: 'var(--border-emphasis)',
  BorderMuted: 'var(--border-muted)',
  BorderFaded: 'var(--border-faded)',
  BorderCritical: 'var(--border-critical)',
  BorderSuccess: 'var(--border-success)',
  BorderWarning: 'var(--border-warning)',
  FgAction: 'var(--fg-action)',
  FgAttention: 'var(--fg-attention)',
  FgCritical: 'var(--fg-critical)',
  FgCriticalMuted: 'var(--fg-critical-muted)',
  FgBlack: 'var(--fg-black)',
  FgFaded: 'var(--fg-faded)',
  FgNeutral: 'var(--fg-neutral)',
  FgMuted: 'var(--fg-muted)',
  FgDisabled: 'var(--fg-disabled)',
  FgSuccess: 'var(--fg-success)',
} as const;

export type ThemeVariableValue = (typeof ThemeVariable)[keyof typeof ThemeVariable];
