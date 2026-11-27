import './style.scss';

type Props = {
  code: string;
};

export const VerificationCode = ({ code }: Props) => {
  return (
    <div className="verification-code" data-testid="verification-code">
      {Array.from(code).map((char, index) => (
        <span className="digit" key={index}>
          {char}
        </span>
      ))}
    </div>
  );
};
