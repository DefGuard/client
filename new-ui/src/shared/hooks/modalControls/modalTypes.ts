import z from 'zod';

export const ModalName = {
  Test: 'test',
} as const;

export type ModalNameValue = (typeof ModalName)[keyof typeof ModalName];

const modalOpenArgsSchema = z.discriminatedUnion('name', [
  z.object({
    name: z.literal(ModalName.Test),
    data: z.object({
      test: z.string(),
    }),
  }),
]);

export type ModalOpenEvent = z.infer<typeof modalOpenArgsSchema>;
