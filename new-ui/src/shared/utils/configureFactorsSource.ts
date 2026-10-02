import { getCurrentWindow } from '@tauri-apps/api/window';
import { WindowId } from '../consts';
import {
  ConfigureFactorsSource,
  type ConfigureFactorsSourceValue,
} from '../rust-api/types';

/**
 * Source for a connect button asking for the Configure MFA screen, read off the window it is
 * rendered in, so the same card reports itself correctly in either view.
 */
export const connectConfigureFactorsSource = (): ConfigureFactorsSourceValue =>
  getCurrentWindow().label === WindowId.FullView
    ? ConfigureFactorsSource.FullConnect
    : ConfigureFactorsSource.TrayConnect;

export const mfaEditConfigureFactorsSource = (): ConfigureFactorsSourceValue =>
  getCurrentWindow().label === WindowId.FullView
    ? ConfigureFactorsSource.FullMfaEdit
    : ConfigureFactorsSource.TrayMfaEdit;
