import cardImage from '../assets/location_avatar.png';

export const LocationCardIcon = () => {
  return (
    <div
      className="location-card-icon"
      style={{
        display: 'inline-flex',
        flexFlow: 'row',
        alignItems: 'center',
        justifyContent: 'center',
        height: 40,
        width: 40,
        overflow: 'hidden',
      }}
    >
      <img loading="eager" height={40} width={40} src={cardImage} />
    </div>
  );
};
