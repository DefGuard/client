import z from 'zod';
import type { OpenUpdateInstanceModalData, OpenUpdateTunnelModalData } from './types';

export const ModalName = {
  UpdateInstance: 'update-instance',
  UpdateTunnel: 'update-tunnel',
} as const;

export type ModalNameValue = (typeof ModalName)[keyof typeof ModalName];

const modalOpenArgsSchema = z.discriminatedUnion('name', [
  z.object({
    name: z.literal(ModalName.UpdateInstance),
    data: z.custom<OpenUpdateInstanceModalData>(),
  }),
  z.object({
    name: z.literal(ModalName.UpdateTunnel),
    data: z.custom<OpenUpdateTunnelModalData>(),
  }),
]);

export type ModalOpenEvent = z.infer<typeof modalOpenArgsSchema>;
