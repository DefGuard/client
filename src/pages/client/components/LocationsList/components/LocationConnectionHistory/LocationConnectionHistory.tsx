import byteSize from 'byte-size';
import dayjs from 'dayjs';
import { floor } from 'lodash-es';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import {
  ListHeader,
  ListRowCell,
  ListSortDirection,
} from '../../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import { Connection } from '../../../../types';

type Props = {
  connections: Connection[];
};

const getDuration = (start: string, end: string): string => {
  const minutes = dayjs.utc(end).diff(dayjs.utc(start), 'm');
  if (minutes > 60) {
    const hours = floor(minutes / 60);
    const res = [`${hours}h`];
    if (minutes % 60 > 0) {
      res.push(`${minutes % 60}m`);
    }
    return res.join(' ');
  } else {
    return `${minutes}m`;
  }
};

export const LocationConnectionHistory = ({ connections }: Props) => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.detailView.history.headers;
  const listHeaders = useMemo((): ListHeader[] => {
    return [
      {
        text: pageLL.date(),
        key: 'date',
        sortDirection: ListSortDirection.DESC,
        sortable: true,
        active: true,
      },
      {
        text: pageLL.duration(),
        key: 'duration',
      },
      {
        text: pageLL.connectedFrom(),
        key: 'connected_from',
      },
      {
        text: pageLL.upload(),
        key: 'upload',
      },

      {
        text: pageLL.download(),
        key: 'download',
      },
    ];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const listCells = useMemo((): ListRowCell<Connection>[] => {
    const allCells = [
      {
        key: 'date',
        render: (connection: Connection) => (
          <span className="date">
            {`${dayjs.utc(connection.start).local().format('DD.MM.YYYY')} : 
            ${dayjs.utc(connection.start).local().format('HH.mm')} - 
            ${dayjs.utc(connection.end).local().format('HH.mm')} `}
          </span>
        ),
      },
      {
        key: 'duration',
        render: (connection: Connection) => (
          <span className="duration">
            {`${getDuration(connection.start, connection.end)}`}
          </span>
        ),
      },
      {
        key: 'connected_from',
        render: (connection: Connection) => (
          <span className="connected-from">{connection.connected_from}</span>
        ),
      },
      {
        key: 'upload',
        render: (connection: Connection) => (
          <span className="upload">{`${byteSize(connection.upload)}`}</span>
        ),
      },

      {
        key: 'download',
        render: (connection: Connection) => (
          <span className="download">{`${byteSize(connection.download)}`}</span>
        ),
      },
    ];
    return allCells;
  }, []);

  const getListPadding = useMemo(() => {
    return {
      left: 20,
      right: 20,
    };
  }, []);

  return (
    <VirtualizedList
      className="connections-list"
      rowSize={70}
      data={connections}
      headers={listHeaders}
      cells={listCells}
      headerPadding={{
        left: 15,
        right: 15,
      }}
      padding={getListPadding}
    />
  );
};
