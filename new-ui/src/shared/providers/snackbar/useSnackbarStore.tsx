import { Subject } from 'rxjs';
import { create } from 'zustand';
import type { SnackbarConfig, UpdateSnackbar } from './types';

interface StoreValues {
  snackSubject: Subject<SnackbarConfig>;
  updateSubject: Subject<UpdateSnackbar>;
  closeSubject: Subject<string>;
  clearSubject: Subject<void>;
}

interface Store extends StoreValues {}

const defaults: StoreValues = {
  snackSubject: new Subject(),
  updateSubject: new Subject(),
  closeSubject: new Subject(),
  clearSubject: new Subject(),
};

export const useSnackbarStore = create<Store>(() => ({ ...defaults }));
