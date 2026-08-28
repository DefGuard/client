import { useQuery } from '@tanstack/react-query';

type Props = {
  src: string;
  poster?: string;
};

export const SlideVideo = ({ src, poster }: Props) => {
  // Bundled assets are served without range request support, which makes video
  // playback stutter. Play it from an in-memory blob instead.
  // https://github.com/orgs/tauri-apps/discussions/7870
  const { data: blobSrc } = useQuery({
    queryKey: ['slide-video', src] as const,
    queryFn: async () => {
      const response = await fetch(src);
      return URL.createObjectURL(await response.blob());
    },
  });

  return (
    <video
      src={blobSrc}
      poster={poster}
      width={'100%'}
      height={'auto'}
      autoPlay
      loop
      muted
      playsInline
      style={{ overflow: 'hidden' }}
    />
  );
};
