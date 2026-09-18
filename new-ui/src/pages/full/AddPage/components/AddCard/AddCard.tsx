import './style.scss';
import clsx from 'clsx';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonSize, ButtonVariant } from '../../../../../shared/components/Button/types';
import defaultImage from './assets/image_1.png';
import yubiImage from './assets/image_2.png';
import wireguardImage from './assets/image_3.png';
import lockImage from './assets/image_4.png';

interface Props {
  actionText: string;
  title: string;
  description: string;
  image: 'default' | 'yubi' | 'wireguard' | 'lock';
  loading?: boolean;
  onClick?: () => void;
}

export const AddCard = ({
  actionText,
  description,
  title,
  image,
  loading = false,
  onClick,
}: Props) => {
  const renderImage = () => {
    switch (image) {
      case 'default':
        return (
          <img
            src={defaultImage}
            width={274}
            height={261}
            decoding="async"
            loading="eager"
          />
        );
      case 'wireguard':
        return (
          <img
            src={wireguardImage}
            width={270.45}
            height={312.34}
            decoding="async"
            loading="eager"
          />
        );
      case 'yubi':
        return (
          <img
            src={yubiImage}
            width={460.07}
            height={377.69}
            decoding="async"
            loading="eager"
          />
        );
      case 'lock':
        return (
          <img
            src={lockImage}
            width={165.57}
            height={219.5}
            decoding="async"
            loading="eager"
          />
        );
    }
  };

  return (
    <div className={clsx('add-card', `image-${image}`)}>
      <div className="track">
        <div className="images">{renderImage()}</div>
        <div className="contents">
          <p className="title">{title}</p>
          <p className="description">{description}</p>
          <Button
            loading={loading}
            size={ButtonSize.Primary}
            variant={ButtonVariant.Secondary}
            text={actionText}
            onClick={onClick}
          />
        </div>
      </div>
    </div>
  );
};
