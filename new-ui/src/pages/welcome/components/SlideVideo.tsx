type Props = {
  src: string;
};

export const SlideVideo = ({ src }: Props) => {
  return (
    <video
      src={src}
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
