import './style.scss';

import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import QRCode from 'react-qr-code';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Input } from '../../../../../../../../shared/defguard-ui/components/Layout/Input/Input';
import { MessageBox } from '../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { Select } from '../../../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../../../../shared/defguard-ui/components/Layout/Select/types';
import SvgIconHamburgerDotted from '../../../../../../../../shared/defguard-ui/components/svg/IconHamburgerDotted';
import { useTheme } from '../../../../../../../../shared/defguard-ui/hooks/theme/useTheme';
import { DeviceConfig } from '../../../../../../../../shared/hooks/api/types';
import { downloadWGConfig } from '../../../../../../../../shared/utils/downloadWGConfig';
import { useEnrollmentStore } from '../../../../../../hooks/store/useEnrollmentStore';

const networkIdentifier = (c: DeviceConfig): number => c.network_id;

const renderSelected = (c: DeviceConfig): SelectSelectedValue => ({
  key: c.network_id,
  displayValue: c.network_name,
});

export const DeviceConfiguration = () => {
  const { colors } = useTheme();
  const [selected, setSelected] = useState<DeviceConfig | undefined>();

  const { LL } = useI18nContext();

  const cardLL = LL.pages.enrollment.steps.deviceSetup.cards.device.config;

  const deviceState = useEnrollmentStore((state) => state.deviceState);

  const autoMode = !isUndefined(deviceState?.device?.privateKey);

  const selectOptions = useMemo(
    (): SelectOption<DeviceConfig>[] =>
      deviceState?.configs?.map((c) => ({
        value: c,
        label: c.network_name,
        key: c.network_id,
      })) ?? [],
    [deviceState?.configs],
  );

  const preparedConfig = useMemo(() => {
    if (deviceState?.device?.privateKey) {
      return selected?.config.replace('YOUR_PRIVATE_KEY', deviceState.device.privateKey);
    }

    if (deviceState?.device?.pubkey) {
      return selected?.config.replace('YOUR_PRIVATE_KEY', deviceState.device.pubkey);
    }

    return selected?.config;
  }, [selected, deviceState?.device?.privateKey, deviceState?.device?.pubkey]);

  useEffect(() => {
    if (deviceState?.configs && deviceState.configs.length) {
      setSelected(deviceState.configs[0]);
    }
  }, [deviceState?.configs]);

  return (
    <>
      <MessageBox
        message={parse(autoMode ? cardLL.messageBox.auto() : cardLL.messageBox.manual())}
      />
      <Input
        value={deviceState?.device?.name}
        label={cardLL.deviceNameLabel()}
        disabled
        onChange={(e) => {
          e.preventDefault();
          e.stopPropagation();
          return;
        }}
      />

      <div className="qr-info">
        <p>{cardLL.cardTitle()}</p>
      </div>

      <Card id="device-config-card">
        <div className="top">
          <SvgIconHamburgerDotted />
          <label>{cardLL.card.selectLabel()}:</label>
          <Select
            renderSelected={renderSelected}
            identify={networkIdentifier}
            options={selectOptions}
            onChangeSingle={(config) => setSelected(config)}
            selected={selected}
          />
          <div className="actions">
            <ActionButton variant={ActionButtonVariant.QRCODE} active />
            <ActionButton
              variant={ActionButtonVariant.COPY}
              disabled={isUndefined(selected) || !window.isSecureContext}
              onClick={() => {
                if (selected && window.isSecureContext) {
                  if (deviceState?.device?.privateKey && preparedConfig) {
                    navigator.clipboard
                      .writeText(preparedConfig)
                      .catch((e) => console.error(e));
                  } else {
                    navigator.clipboard
                      .writeText(selected.config)
                      .catch((e) => console.error(e));
                  }
                }
              }}
            />
            <ActionButton
              disabled={isUndefined(selected)}
              variant={ActionButtonVariant.DOWNLOAD}
              onClick={() => {
                if (preparedConfig && selected) {
                  downloadWGConfig(
                    deviceState?.device?.privateKey ? preparedConfig : selected.config,
                    `${selected.network_name.toLowerCase().replace(/\s+/g, '-')}`,
                  );
                }
              }}
            />
          </div>
        </div>
        <div className="qr">
          {!isUndefined(preparedConfig) && (
            <QRCode
              size={275}
              value={preparedConfig}
              bgColor={colors.surfaceDefaultModal}
              fgColor={colors.textBodyPrimary}
            />
          )}
        </div>
      </Card>
    </>
  );
};
