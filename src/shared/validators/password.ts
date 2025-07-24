import { z } from 'zod';

import type { TranslationFunctions } from '../../i18n/i18n-types';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
} from '../patterns';

export const passwordValidator = (LL: TranslationFunctions) =>
  z
    .string()
    .trim()
    .nonempty(LL.form.errors.required())
    .min(8, LL.form.errors.minLength({ length: 8 }))
    .max(128, LL.form.errors.maxLength({ length: 128 }))
    .regex(patternAtLeastOneDigit, LL.form.errors.numberRequired())
    .regex(patternAtLeastOneSpecialChar, LL.form.errors.specialsRequired())
    .regex(patternAtLeastOneLowerCaseChar, LL.form.errors.oneLower())
    .regex(patternAtLeastOneUpperCaseChar, LL.form.errors.oneUpper());
