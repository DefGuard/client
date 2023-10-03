import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { error, info } from 'tauri-plugin-log-api';

import { useI18nContext } from '../../i18n/i18n-react';
import { Select } from '../../shared/defguard-ui/components/Layout/Select/Select';
import { SelectSelectedValue } from '../../shared/defguard-ui/components/Layout/Select/types';
import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { LocationsList } from './components/LocationsList/LocationsList';
import { AddInstanceModal } from './components/modals/AddInstanceModal/AddInstanceModal';
import { useClientStore } from './hooks/useClientStore';
import { clientQueryKeys } from './query';

const { getInstances } = clientApi;

enum LayoutType {
  GRID = 'GRID',
  DETAIL = 'DETAIL',
}

export const ClientPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client;
  const setInstances = useClientStore((state) => state.setInstances);
  const [layoutType, setLayoutType] = useState(LayoutType.GRID);

  useQuery({
    queryKey: [clientQueryKeys.getInstances],
    queryFn: getInstances,
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    onSuccess: (res) => {
      setInstances(res);
      info('Retrieved instances');
    },
    onError: (err) => {
      error(String(err));
    },
  });

  const renderSelected = useCallback((selectedValue: LayoutType): SelectSelectedValue => {
    const options = [
      { value: LayoutType.GRID, label: 'Grid View' },
      { value: LayoutType.DETAIL, label: 'Detail View' },
    ];
    const selectedOption = options.find((option) => option.value === selectedValue);

    if (!selectedOption) {
      return {
        key: 'none',
        displayValue: '',
      };
    }

    return {
      key: selectedOption.value,
      displayValue: selectedOption.label,
    };
  }, []);

  return (
    <>
      <section id="client-page">
        <header>
          <h1>{pageLL.title()}</h1>
          <Select
            renderSelected={renderSelected}
            selected={layoutType}
            options={[
              { key: 'grid', value: LayoutType.GRID, label: 'Grid View' },
              { key: 'detail', value: LayoutType.DETAIL, label: 'Detail View' },
            ]}
            onChangeSingle={(selectedValue) => {
              setLayoutType(selectedValue);
            }}
          />
        </header>
        <LocationsList layoutType={layoutType} />
      </section>
      <ClientSideBar />
      <AddInstanceModal />
    </>
  );
};
