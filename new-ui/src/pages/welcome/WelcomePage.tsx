import { Fragment } from 'react/jsx-runtime';
import { FullPage } from '../../shared/layouts/FullPage/FullPage';
import './style.scss';
import { IconKind } from '../../shared/components/Icon';
import { IconButton } from '../../shared/components/IconButton/IconButton';
import { IconButtonVariant } from '../../shared/components/IconButton/types';
import { api } from '../../shared/rust-api/api';
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
        <header data-tauri-drag-region>
          <h1 data-tauri-drag-region>{`What's new`}</h1>
          <IconButton
            icon={IconKind.Close}
            variant={IconButtonVariant.Big}
            onClick={() => {
              void api.closeWelcomeWindow();
            }}
          />
        </header>
        <WelcomeCarousel slides={welcomeSlides} />
      </FullPage>
    </Fragment>
  );
};
