type Props = {
  src: string;
  poster?: string;
};

export const SlideVideo = ({ src, poster }: Props) => {
  return (
    <video
      src={src}
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
