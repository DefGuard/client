import './style.scss';

type Props = {
  height: string | number;
  width?: string | number;
};

/**Spawns a block with a strict size, meant to fill spaces that are not regular like layouts that can't utilize "gap" css property due to irregular gaps in across same axis*/
export const SizedBox = ({ width, height }: Props) => {
  return (
    <div
      className="sized-box"
      style={{
        width: width ?? '100%',
        height,
      }}
    ></div>
  );
};
