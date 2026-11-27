export const BadgeVariant = {
  Default: 'default',
  Success: 'success',
  Critical: 'critical',
} as const;

export type BadgeVariantValue = (typeof BadgeVariant)[keyof typeof BadgeVariant];
