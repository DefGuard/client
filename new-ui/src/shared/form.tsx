import { createFormHook } from '@tanstack/react-form';
import { FormInput } from './components/form/FormInput/FormInput';
import { fieldContext, formContext } from './form-context';

export { useFieldContext, useFormContext } from './form-context';

export const { useAppForm, withFieldGroup, withForm } = createFormHook({
  fieldContext,
  formContext,
  fieldComponents: {
    FormInput,
  },
  formComponents: {},
});
