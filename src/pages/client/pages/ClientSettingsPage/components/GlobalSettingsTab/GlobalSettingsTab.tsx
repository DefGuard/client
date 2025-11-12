import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  type SubmitHandler,
  type UseControllerProps,
  useController,
  useForm,
} from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import {
  type SelectOption,
  type SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import {
  availableThemes,
  type ThemeKey,
} from '../../../../../../shared/defguard-ui/hooks/theme/types';
import {
  type AppConfig,
  availableLogLevels,
  availableTrayThemes,
  type LogLevel,
  type TrayIconTheme,
} from '../../../../clientAPI/types';
import { useClientStore } from '../../../../hooks/useClientStore';

type FormFields = AppConfig;

type FormMemberProps = {
  controller: UseControllerProps<FormFields>;
};

export const GlobalSettingsTab = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;
  const currentConfig = useClientStore((s) => s.appConfig);
  const setAppConfig = useClientStore((s) => s.updateAppConfig, shallow);

  const { mutateAsync } = useMutation({
    mutationFn: setAppConfig,
  });

  const schema = useMemo(
    () =>
      z.object({
        theme: z
          .string()
          .min(1, LL.form.errors.required())
          .refine(
            (value) => availableThemes.includes(value as ThemeKey),
            LL.form.errors.invalid(),
          ),
        log_level: z
          .string()
          .min(1, LL.form.errors.required())
          .refine((v) => availableLogLevels.includes(v as LogLevel)),
        tray_theme: z
          .string()
          .min(1, LL.form.errors.required())
          .refine((v) => availableTrayThemes.includes(v as TrayIconTheme)),
        check_for_updates: z.boolean(),
        peer_alive_period: z
          .number({
            invalid_type_error: LL.form.errors.required(),
            required_error: LL.form.errors.required(),
          })
          .gte(120, LL.form.errors.minValue({ min: 120 })),
        // TODO nullable number
        mtu: z.coerce.number(),
        // mtu: z
        //   .coerce
        //   .number()
        //   .lte(65535, LL.form.errors.maxValue({ max: 65535 }))
        //   .transform((val) => val === 0 ? null : val)
        //   .optional(),
        // mtu: z.preprocess(
        //   (val) =>
        //     val == null || (typeof val === 'string' && val.trim() === '')
        //       ? null
        //       : Number(val),
        //   z.nullable(
        //     z.number().lte(65535, LL.form.errors.maxValue({ max: 65535 })),
        //   ),
        // ),
        // mtu: z
        //   .union([
        //     z.coerce.number().lte(65535, LL.form.errors.maxValue({ max: 65535 })),
        //     z.null(),
        //   ])
        //   .transform((val) => (val === 0 ? null : val))
        //   .optional()
        // mtu: z.union([
        //   z.literal(""), // 1. Explicitly allow an empty string
        //   z.coerce // 2. Or, allow a number...
        //     .number()
        //     .lte(65535, LL.form.errors.maxValue({ max: 65535 })),
        // ])
        //   .optional() // 3. Still allow the field to be undefined
        //   .transform((val) => (val === "" ? null : val))
        // mtu: z.union([
        //   z.string().optional(),
        //   z.coerce.number().lte(65535, LL.form.errors.maxValue({ max: 65535 }))
        // ])
        //   .transform((val) => (val === 0 || val === null ? null : val))
        //   .optional()
      }),
    [LL.form.errors],
  );

  const {
    handleSubmit,
    control,
    reset,
    formState: { isDirty, isValid },
  } = useForm<AppConfig>({
    defaultValues: currentConfig,
    mode: 'all',
    resolver: zodResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    const newConfig = await mutateAsync(values);
    reset(newConfig);
  };

  return (
    <form id="global-settings-tab" onSubmit={handleSubmit(handleValidSubmit)}>
      <div className="controls spaced">
        <Button
          type="submit"
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          disabled={!isDirty || !isValid}
          text={LL.common.controls.save()}
        />
      </div>
      <section className="spaced">
        <h2>{localLL.versionUpdate.title()}</h2>
        <CheckForUpdatesOption controller={{ control, name: 'check_for_updates' }} />
      </section>
      <section>
        <h2>{localLL.tray.title()}</h2>
        <TrayIconThemeSelect controller={{ control, name: 'tray_theme' }} />
      </section>
      <section>
        <h2>{localLL.logging.title()}</h2>
        <LoggingLevelSelect controller={{ control, name: 'log_level' }} />
      </section>
      <section>
        <h2>{localLL.theme.title()}</h2>
        <ThemeSelect controller={{ control, name: 'theme' }} />
      </section>
      <section>
        <header>
          <h2>
            {localLL.peer_alive.title()} <span>{localLL.common.value_in_seconds()}</span>
          </h2>
          <Helper initialPlacement="right">
            <p>{localLL.peer_alive.helper()}</p>
          </Helper>
        </header>
        <FormInput controller={{ control, name: 'peer_alive_period' }} type="number" />
      </section>
      <section>
        <header>
          <h2>{localLL.mtu.title()}</h2>
          <Helper initialPlacement="right">
            <p>{localLL.mtu.helper()}</p>
          </Helper>
        </header>
        <FormInput controller={{ control, name: 'mtu' }} type="number" />
      </section>
    </form>
  );
};

const ThemeSelect = ({ controller }: FormMemberProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.theme;

  const options = useMemo((): SelectOption<ThemeKey>[] => {
    const res: SelectOption<ThemeKey>[] = [
      {
        key: 0,
        label: localLL.options.light(),
        value: 'light',
      },
      {
        key: 1,
        label: localLL.options.dark(),
        value: 'dark',
      },
    ];
    return res;
  }, [localLL.options]);

  const renderSelected = useCallback(
    (theme: ThemeKey): SelectSelectedValue => {
      const option = options.find((o) => o.value === theme);
      if (option) {
        return {
          key: option.key,
          displayValue: option.label,
        };
      }
      return {
        key: 999,
        displayValue: '',
      };
    },
    [options],
  );

  return (
    <FormSelect
      options={options}
      renderSelected={renderSelected}
      controller={controller}
    />
  );
};

const LoggingLevelSelect = ({ controller }: FormMemberProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.logging;
  const appConfig = useClientStore((state) => state.appConfig);
  const [showWarning, setShowWarning] = useState(false);
  const {
    field: { value },
  } = useController(controller);

  const loggingOptions = useMemo((): SelectOption<LogLevel>[] => {
    const res: SelectOption<LogLevel>[] = [
      {
        key: 0,
        label: localLL.options.error(),
        value: 'ERROR',
      },
      {
        key: 1,
        label: localLL.options.info(),
        value: 'INFO',
      },
      {
        key: 2,
        label: localLL.options.debug(),
        value: 'DEBUG',
      },
      {
        key: 3,
        label: localLL.options.trace(),
        value: 'TRACE',
      },
    ];
    return res;
  }, [localLL.options]);

  const renderSelected = useCallback(
    (val: LogLevel) => {
      const option = loggingOptions.find((o) => o.value === val);
      if (option) {
        return {
          key: option.key,
          displayValue: option.label,
        };
      }
      return {
        key: 999,
        displayValue: '',
      };
    },
    [loggingOptions],
  );

  useEffect(() => {
    if (value !== appConfig.log_level) {
      setShowWarning(true);
    }
  }, [value, appConfig.log_level]);

  return (
    <>
      {showWarning && <p className="warning-message">{localLL.warning()}</p>}
      <FormSelect
        controller={controller}
        sizeVariant={SelectSizeVariant.STANDARD}
        options={loggingOptions}
        renderSelected={renderSelected}
      />
    </>
  );
};

const TrayIconThemeSelect = ({ controller }: FormMemberProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;

  const trayThemeSelectOptions = useMemo((): SelectOption<TrayIconTheme>[] => {
    const res: SelectOption<TrayIconTheme>[] = [
      {
        label: localLL.tray.options.color(),
        value: 'color',
        key: 0,
      },
      {
        value: 'gray',
        key: 1,
        label: localLL.tray.options.gray(),
      },
      {
        value: 'white',
        key: 2,
        label: localLL.tray.options.white(),
      },
      {
        value: 'black',
        key: 3,
        label: localLL.tray.options.black(),
      },
    ];
    return res;
  }, [localLL.tray.options]);

  const renderSelectedTrayTheme = useCallback(
    (theme: TrayIconTheme): SelectSelectedValue => {
      const option = trayThemeSelectOptions.find((t) => t.value === theme);
      if (option) {
        return {
          key: option.key,
          displayValue: option.label,
        };
      }
      return {
        key: 'color',
        displayValue: 'color',
      };
    },
    [trayThemeSelectOptions],
  );

  return (
    <FormSelect
      sizeVariant={SelectSizeVariant.STANDARD}
      options={trayThemeSelectOptions}
      label={localLL.tray.label()}
      renderSelected={renderSelectedTrayTheme}
      controller={controller}
    />
  );
};

const CheckForUpdatesOption = ({ controller }: FormMemberProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;

  return (
    <FormCheckBox
      labelPlacement="right"
      label={localLL.versionUpdate.checkboxTitle()}
      controller={controller}
    />
  );
};
