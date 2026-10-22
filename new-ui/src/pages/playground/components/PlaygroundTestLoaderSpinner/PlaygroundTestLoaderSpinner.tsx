import './style.scss';
import { LoaderSpinner } from '../../../../shared/components/LoaderSpinner/LoaderSpinner';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const sizes = [20, 24, 32, 40, 48, 64];
const thicknesses = [1, 2, 3, 4];
const thicknessPreviewSize = 48;

export const PlaygroundTestLoaderSpinner = () => {
  return (
    <PlaygroundCard>
      <div className="playground-test-loader-spinner">
        <h3>LoaderSpinner</h3>
        <div className="row">
          {sizes.map((size) => (
            <div className="item" key={size}>
              <LoaderSpinner variant="primary" size={size} />
              <span>{size}</span>
            </div>
          ))}
        </div>
        <div className="row">
          {thicknesses.map((thickness) => (
            <div className="item" key={thickness}>
              <LoaderSpinner
                variant="primary"
                size={thicknessPreviewSize}
                thickness={thickness}
              />
              <span>{`${thickness}px`}</span>
            </div>
          ))}
        </div>
      </div>
    </PlaygroundCard>
  );
};
