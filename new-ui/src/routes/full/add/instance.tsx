import { createFileRoute } from '@tanstack/react-router';
import { hostname } from '@tauri-apps/plugin-os';
import { z } from 'zod';
import { AddInstancePage } from '../../../pages/full/AddInstancePage/AddInstancePage';

const searchSchema = z.object({
  token: z.string().optional(),
  url: z.string().optional(),
});

export const Route = createFileRoute('/full/add/instance')({
  validateSearch: searchSchema,
  loader: async () => {
    const deviceName = await hostname();
    return {
      deviceName: deviceName ?? '',
    };
  },
  component: AddInstancePage,
});
