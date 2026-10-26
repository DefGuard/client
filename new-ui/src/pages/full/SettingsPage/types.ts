export const SettingsTab = {
  General: 'general',
  Startup: 'startup',
} as const;

export type SettingsTabValue = (typeof SettingsTab)[keyof typeof SettingsTab];
