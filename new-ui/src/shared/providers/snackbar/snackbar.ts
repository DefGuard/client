import { type SnackbarConfig, SnackbarVariant, type SnackbarVariantValue } from './types';
import { useSnackbarStore } from './useSnackbarStore';

export class SnackbarAnchor {
  readonly id: string;

  constructor(id: string) {
    this.id = id;
  }

  dismiss() {
    useSnackbarStore.getState().closeSubject.next(this.id);
  }

  update(update: Partial<Omit<SnackbarConfig, 'id'>>, resetAutoDismissTimer?: boolean) {
    useSnackbarStore.getState().updateSubject.next({
      id: this.id,
      update,
      resetAutoDismiss: resetAutoDismissTimer,
    });
  }
}

type CustomSpawnArg = Omit<SnackbarConfig, 'id' | 'variant'> & {
  id: string;
  variant: SnackbarVariantValue;
};

export const Snackbar = {
  default: (text: string) => {
    useSnackbarStore.getState().snackSubject.next({
      text,
      variant: SnackbarVariant.Default,
    });
  },
  success: (text: string) => {
    useSnackbarStore.getState().snackSubject.next({
      text,
      variant: SnackbarVariant.Success,
    });
  },
  warning: (text: string) => {
    useSnackbarStore.getState().snackSubject.next({
      text,
      variant: SnackbarVariant.Warning,
    });
  },
  error: (text: string) => {
    useSnackbarStore.getState().snackSubject.next({
      text,
      variant: SnackbarVariant.Error,
    });
  },
  loading: (text: string, id: string) => {
    const anchor = new SnackbarAnchor(id);
    useSnackbarStore.getState().snackSubject.next({
      id,
      text,
      variant: SnackbarVariant.Loading,
    });
    return anchor;
  },
  custom: (customProps: CustomSpawnArg) => {
    const anchor = new SnackbarAnchor(customProps.id);
    useSnackbarStore.getState().snackSubject.next(customProps);
    return anchor;
  },
  clear: () => {
    useSnackbarStore.getState().clearSubject.next();
  },
} as const;
