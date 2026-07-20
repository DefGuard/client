import { useMutation } from '@tanstack/react-query';
import { error as logError } from '@tauri-apps/plugin-log';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Modal } from '../../../../../shared/components/Modal/Modal';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { Split } from '../../../../../shared/components/Split/Split';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import type { OpenUpdateTunnelModalData } from '../../../../../shared/hooks/modalControls/types';
import { Snackbar } from '../../../../../shared/providers/snackbar/snackbar';
import { api } from '../../../../../shared/rust-api/api';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  patternValidIpV6WithMask,
  patternValidIpWithMask,
} from '../../../../../shared/utils/patterns';
import {
  allowedIpsSchema,
  endpointSchema,
  optionalWireguardKeySchema,
  wireguardKeySchema,
} from '../../../../../shared/utils/zod';

const modalNameKey = ModalName.UpdateTunnel;

export const UpdateTunnelModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<OpenUpdateTunnelModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      title="Edit tunnel"
      id="update-tunnel-modal"
      size="primary"
      isOpen={isOpen}
      onClose={() => {
        setOpen(false);
      }}
    >
      {isOpen && isPresent(modalData) && <ModalContent data={modalData} />}
    </Modal>
  );
};

const formSchema = z.object({
  name: z.string().trim().min(1, 'Field is required'),
  address: z.string().refine((value) => {
    if (!value) return false;
    return value
      .split(',')
      .map((ip) => ip.trim())
      .every(
        (ip) => patternValidIpWithMask.test(ip) || patternValidIpV6WithMask.test(ip),
      );
  }, 'Field is invalid'),
  prvkey: wireguardKeySchema,
  pubkey: wireguardKeySchema,
  server_pubkey: wireguardKeySchema,
  preshared_key: optionalWireguardKeySchema,
  endpoint: endpointSchema,
  dns: z.string(),
  allowed_ips: allowedIpsSchema,
  persistent_keep_alive: z.number().int().min(0),
  pre_up: z.string(),
  post_up: z.string(),
  pre_down: z.string(),
  post_down: z.string(),
});

type FormFields = z.infer<typeof formSchema>;

const ModalContent = ({ data }: { data: OpenUpdateTunnelModalData }) => {
  const { mutateAsync: updateTunnel, isPending } = useMutation({
    mutationFn: api.updateTunnel,
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      name: data.name,
      address: data.address,
      prvkey: data.prvkey,
      pubkey: data.pubkey,
      server_pubkey: data.server_pubkey,
      preshared_key: data.preshared_key ?? '',
      endpoint: data.endpoint,
      dns: data.dns ?? '',
      allowed_ips: data.allowed_ips ?? '',
      persistent_keep_alive: data.persistent_keep_alive,
      pre_up: data.pre_up ?? '',
      post_up: data.post_up ?? '',
      pre_down: data.pre_down ?? '',
      post_down: data.post_down ?? '',
    }),
    [data],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      try {
        await updateTunnel({
          id: data.id as number,
          route_all_traffic: data.route_all_traffic,
          ...value,
        });
        Snackbar.default('Tunnel updated.');
        closeModal(modalNameKey);
      } catch (e) {
        void logError(`Tunnel update failed: ${e}`);
        Snackbar.error('Failed to update tunnel.');
      }
    },
  });

  return (
    <div className="update-tunnel-modal">
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Tunnel name" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="address">
            {(field) => <field.FormInput required label="Address" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <Split>
            <form.AppField name="pubkey">
              {(field) => <field.FormInput required label="Public key" />}
            </form.AppField>
            <form.AppField name="prvkey">
              {(field) => (
                <field.FormInput required label="Private key" type="password" />
              )}
            </form.AppField>
          </Split>
          <SizedBox height={ThemeSpacing.Xl} />
          <Split>
            <form.AppField name="server_pubkey">
              {(field) => <field.FormInput required label="VPN server public key" />}
            </form.AppField>
            <form.AppField name="preshared_key">
              {(field) => <field.FormInput label="Preshared key" type="password" />}
            </form.AppField>
          </Split>
          <SizedBox height={ThemeSpacing.Xl} />
          <Split>
            <form.AppField name="endpoint">
              {(field) => <field.FormInput required label="VPN server address:port" />}
            </form.AppField>
            <form.AppField name="dns">
              {(field) => <field.FormInput label="DNS" />}
            </form.AppField>
          </Split>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="allowed_ips">
            {(field) => (
              <field.FormInput label="Allowed IPs (add multiple separated by coma)" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="persistent_keep_alive">
            {(field) => <field.FormInput required label="Persistent keep alive (sec)" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <Split>
            <form.AppField name="pre_up">
              {(field) => <field.FormInput label="Pre-up" />}
            </form.AppField>
            <form.AppField name="post_up">
              {(field) => <field.FormInput label="Post-up" />}
            </form.AppField>
          </Split>
          <SizedBox height={ThemeSpacing.Xl} />
          <Split>
            <form.AppField name="pre_down">
              {(field) => <field.FormInput label="Pre-down" />}
            </form.AppField>
            <form.AppField name="post_down">
              {(field) => <field.FormInput label="Post-down" />}
            </form.AppField>
          </Split>
          <Controls>
            <div className="right">
              <Button
                variant={ButtonVariant.Secondary}
                text="Cancel"
                onClick={() => {
                  closeModal(modalNameKey);
                }}
              />
              <form.Subscribe selector={(s) => s.isSubmitting}>
                {(isSubmitting) => (
                  <Button
                    variant={ButtonVariant.Primary}
                    text="Update"
                    loading={isSubmitting || isPending}
                    onClick={() => {
                      form.handleSubmit();
                    }}
                  />
                )}
              </form.Subscribe>
            </div>
          </Controls>
        </form.AppForm>
      </form>
    </div>
  );
};
