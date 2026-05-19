import './style.scss';
import { QRCodeCanvas } from 'qrcode.react';

interface Props {
  value: string;
  size?: number;
}

export const QrCard = ({ value, size = 200 }: Props) => {
  return (
    <div className="qr-code-display">
      <QRCodeCanvas value={value} size={size} />
    </div>
  );
};
