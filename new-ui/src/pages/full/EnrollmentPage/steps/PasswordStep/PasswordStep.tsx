/** biome-ignore-all lint/style/noNonNullAssertion: ensured by wizard page */
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import clsx from 'clsx';
import { useCallback } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/shallow';
import { Icon, type IconKindValue } from '../../../../../shared/components/Icon';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { useAppForm, withForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import { api } from '../../../../../shared/rust-api/api';
import { ThemeSpacing } from '../../../../../shared/types';
import { EnrollmentControls } from '../../components/EnrollmentControls/EnrollmentControls';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';
import {
  type PasswordErrorCodeValue,
  passwordErrorMessage,
  refinePasswordField,
} from './utils';

const formSchema = z
  .object({
    password: z.string().trim().min(1, 'Field is required'),
    repeat: z.string().trim(),
  })
  .superRefine(({ password, repeat }, ctx) => {
    const passwordIssues = refinePasswordField(password);
    for (const issue of passwordIssues) {
      ctx.addIssue({
        message: issue,
        code: 'custom',
        continue: true,
        path: ['password'],
      });
    }
    if (repeat.length && repeat !== password) {
      ctx.addIssue({
        message: "Passwords don't match",
        code: 'custom',
        path: ['repeat'],
        continue: true,
      });
    }
  });

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  password: '',
  repeat: '',
};

export const PasswordStep = () => {
  const [proxyUrl, cookie] = useEnrollmentStore(
    useShallow((s) => [s.proxyUrl!, s.sessionCookie!]),
  );

  const { mutateAsync } = useMutation({
    mutationFn: (password: string) =>
      api.activateUser(proxyUrl, cookie, {
        password,
      }),
    onSuccess: () => {
      useEnrollmentStore.getState().next();
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await mutateAsync(value.password);
    },
  });

  return (
    <div id="password-step" className="step-content">
      <header>
        <h1>Create password</h1>
        <p>
          Please check your data. If something is wrong, notify your administrator after
          you complete the process.
        </p>
      </header>
      <SizedBox height={ThemeSpacing.Xl2} />
      <form
        onSubmit={(event) => {
          event.preventDefault();
          event.stopPropagation();
          form.handleSubmit();
        }}
        autoComplete="off"
      >
        <form.AppForm>
          <form.AppField name="password">
            {(field) => (
              <field.FormInput
                type="password"
                label="Enter new password"
                required
                autocomplete="off"
                mapError={(message) => {
                  const code = message as PasswordErrorCodeValue;
                  return passwordErrorMessage(code);
                }}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="repeat">
            {(field) => (
              <field.FormInput
                type="password"
                label="Confirm new password"
                required
                autocomplete="off"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <CheckList form={form} />
          <form.Subscribe selector={(s) => s.isSubmitting}>
            {(loading) => (
              <EnrollmentControls
                onNext={() => {
                  form.handleSubmit();
                }}
                loading={loading}
              />
            )}
          </form.Subscribe>
        </form.AppForm>
      </form>
    </div>
  );
};

const CheckList = withForm({
  defaultValues: defaultValues,
  render: ({ form }) => {
    const passwordFieldErrors = useStore(
      form.store,
      (state) =>
        (state.fieldMeta.password?.errors as z.core.$ZodIssue[])
          ?.filter((issue) => issue.code === 'custom')
          .map((issue) => issue.message) ?? [],
    );

    const isPasswordClean = useStore(
      form.store,
      (state) => state.fieldMeta.password?.isPristine ?? true,
    );

    const isChecked = useCallback(
      (value: PasswordErrorCodeValue): boolean =>
        !passwordFieldErrors.includes(value) && !isPasswordClean,
      [passwordFieldErrors, isPasswordClean],
    );

    return (
      <div className="checklist">
        <p>Your passwords must include:</p>
        <ul>
          <CheckListItem
            errorCode="password_form_check_minimum"
            checked={isChecked('password_form_check_minimum')}
          />
          <CheckListItem
            errorCode="password_form_check_number"
            checked={isChecked('password_form_check_number')}
          />
          <CheckListItem
            errorCode="password_form_check_special"
            checked={isChecked('password_form_check_special')}
          />
          <CheckListItem
            errorCode="password_form_check_lowercase"
            checked={isChecked('password_form_check_lowercase')}
          />
          <CheckListItem
            errorCode="password_form_check_uppercase"
            checked={isChecked('password_form_check_uppercase')}
          />
        </ul>
      </div>
    );
  },
});

const CheckListItem = ({
  checked,
  errorCode,
}: {
  errorCode: PasswordErrorCodeValue;
  checked: boolean;
}) => {
  const iconKind: IconKindValue = checked ? 'check-filled' : 'empty-point';

  return (
    <li
      className={clsx({
        active: checked,
      })}
    >
      <Icon icon={iconKind} size={16} /> <span>{passwordErrorMessage(errorCode)}</span>
    </li>
  );
};
