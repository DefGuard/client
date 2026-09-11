type Props = {
  src: string;
};

export const SlideImage = ({ src }: Props) => {
  return (
    <img
      src={src}
      loading="eager"
      width={'100%'}
      height={'auto'}
      style={{ overflow: 'hidden' }}
    />
  );
};
