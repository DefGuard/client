import { Fragment } from 'react/jsx-runtime';
import { FullPage } from '../../shared/layouts/FullPage/FullPage';
import './style.scss';
import { IconKind } from '../../shared/components/Icon';
import { IconButton } from '../../shared/components/IconButton/IconButton';
import { IconButtonVariant } from '../../shared/components/IconButton/types';
import { Snackbar } from '../../shared/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../shared/types';
import { WelcomeCarousel } from './components/WelcomeCarousel/WelcomeCarousel';
import { welcomeSlides } from './config';

export const WelcomePage = () => {
  return (
    <Fragment>
      <div
        className="outside-header"
        data-tauri-drag-region
        style={{
          width: '100%',
          height: ThemeSpacing.Md,
        }}
      ></div>
      <FullPage hideScrollContainer id="welcome-page">
        <header>
          <h1>{`What's new`}</h1>
          <IconButton
            icon={IconKind.Close}
            variant={IconButtonVariant.Big}
            onClick={() => {
              Snackbar.default(`Close Welcome view`);
            }}
          />
        </header>
        <WelcomeCarousel slides={welcomeSlides} />
      </FullPage>
    </Fragment>
  );
};
