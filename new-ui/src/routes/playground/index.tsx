import { createFileRoute } from '@tanstack/react-router';
import { PlaygroundIndex } from '../../pages/playground/PlaygroundIndex';

export const Route = createFileRoute('/playground/')({
  component: PlaygroundIndex,
});
