import { createFormHook } from '@tanstack/react-form';
import { fieldContext, formContext } from './form-context';

export { useFieldContext, useFormContext } from './form-context';

export const { useAppForm, withFieldGroup, withForm } = createFormHook({
  fieldContext,
  formContext,
  fieldComponents: {},
  formComponents: {},
});
