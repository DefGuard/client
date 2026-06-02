import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { ThemeSpacing } from '../../../shared/types';
import { FullPage } from '../FullPage/FullPage';
import { AddCard } from './components/AddCard/AddCard';

export const AddPage = () => {
  const navigate = useNavigate();
  return (
    <FullPage id="add-page-view">
      <FullPageTitle title="Add Defguard items" spacing={ThemeSpacing.Xl} />
      <div className="cards">
        <AddCard
          image="default"
          onClick={() => {
            navigate({
              to: '/full/add/instance',
            });
          }}
          title="Add Instance"
          actionText="Add instance"
          description={`Establish a secure connection to your Defguard instance effortlessly by configuring it with a single token—no manual setup.`}
        />
      </div>
    </FullPage>
  );
};
