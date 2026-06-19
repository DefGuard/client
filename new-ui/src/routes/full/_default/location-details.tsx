import { createFileRoute } from '@tanstack/react-router';
import { z } from 'zod';
import { LocationDetailsPage } from '../../../pages/full/LocationDetailsPage/LocationDetailsPage';

const searchSchema = z.object({
  locationId: z.number(),
  locationName: z.string(),
  connectionType: z.enum(['Location', 'Tunnel']),
});

export const Route = createFileRoute('/full/_default/location-details')({
  validateSearch: searchSchema,
  component: LocationDetailsPage,
});
